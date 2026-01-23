//! Timer implementation
//!
//! This module provides both sync and async timer functionality:
//! - Sync timers are mounted on the global object:
//!   - setTimeout/clearTimeout (sync)
//!   - setInterval/clearInterval (sync)
//! - Async timers are mounted under global.timer:
//!   - setTimeout/clearTimeout (async)
//!   - setInterval/clearInterval (async)
//!   - setImmediate (async)
//!
//! # Features
//! - Sync timers for traditional callback-based usage
//! - Async timers that return Promises for modern async/await patterns
//!
//! # Limitations
//! - Unlike Web APIs, this implementation does not support passing additional arguments
//!   to the callback function. Only the callback function and delay are supported.
//! - Delay is in milliseconds and should be a positive number.

use rong::{JSContext, JSFunc, JSResult, JSRuntimeService, JSValue, function::Optional, spawn};

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};
use tokio::sync::Notify;
use tokio::time::Duration;

mod promise;

fn lock_poison<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

#[derive(Clone)]
pub struct TimerRegistry {
    inner: Rc<TimerRegistryInner>,
}

struct TimerEntry {
    notifier: Rc<Notify>,
    // For callback-style timers (global setTimeout/setInterval).
    // We keep the JSFunc in the registry so `on_shutdown` can drop it before the JS runtime is freed,
    // even if the spawned timer task hasn't been polled/canceled yet.
    callback: Option<JSFunc>,
}

struct TimerRegistryInner {
    next_id: AtomicU32,
    timers: Mutex<HashMap<u32, TimerEntry>>,
}

impl JSRuntimeService for TimerRegistry {
    fn on_shutdown(&self) {
        // IMPORTANT: don't block here. This runs during JSRuntime drop on the LocalSet thread.
        // Blocking prevents timer tasks from running and can leak JS GC objects under QuickJS.
        self.shutdown();
    }
}

impl Default for TimerRegistry {
    fn default() -> Self {
        Self {
            inner: Rc::new(TimerRegistryInner {
                next_id: AtomicU32::new(0),
                timers: Mutex::new(HashMap::new()),
            }),
        }
    }
}

impl TimerRegistry {
    fn next_id(&self) -> u32 {
        self.inner.next_id.fetch_add(1, Ordering::Relaxed)
    }

    fn register_timer(&self, id: u32, notifier: Rc<Notify>) {
        lock_poison(&self.inner.timers).insert(
            id,
            TimerEntry {
                notifier,
                callback: None,
            },
        );
    }

    fn register_timer_with_callback(&self, id: u32, notifier: Rc<Notify>, callback: JSFunc) {
        lock_poison(&self.inner.timers).insert(
            id,
            TimerEntry {
                notifier,
                callback: Some(callback),
            },
        );
    }

    fn cancel_timer(&self, id: u32) {
        if let Some(entry) = lock_poison(&self.inner.timers).remove(&id) {
            entry.notifier.notify_waiters();
        } else {
            return;
        }
    }

    fn finish_timer(&self, id: u32) {
        // Best-effort cleanup. It's fine if the timer was already canceled/shutdown.
        let _ = lock_poison(&self.inner.timers).remove(&id);
    }

    fn is_timer_active(&self, id: u32) -> bool {
        lock_poison(&self.inner.timers).contains_key(&id)
    }

    fn get_callback(&self, id: u32) -> Option<JSFunc> {
        lock_poison(&self.inner.timers)
            .get(&id)
            .and_then(|e| e.callback.clone())
    }

    fn shutdown(&self) {
        let mut timers = lock_poison(&self.inner.timers);
        if timers.is_empty() {
            return;
        }

        // Copy the notifiers before draining to avoid deadlock
        let notifiers_copy: Vec<Rc<Notify>> = timers.values().map(|e| e.notifier.clone()).collect();
        timers.clear(); // drops callbacks before the JS runtime is freed

        // Notify all timers outside the lock
        for notifier in notifiers_copy {
            notifier.notify_waiters();
        }
    }
}

fn set_timeout_with_repeat(
    registry: TimerRegistry,
    callback: JSFunc,
    delay: Optional<f64>,
    repeat: bool,
) -> u32 {
    let id = registry.next_id();
    let notifier = Rc::new(Notify::new());

    registry.register_timer_with_callback(id, notifier.clone(), callback);
    let delay = delay.unwrap_or(0.0).max(0.0) as u64;
    let registry_clone = registry.clone();

    spawn(async move {
        // Repeating timers: use `interval.tick()` for all executions.
        // `tokio::time::interval`'s first `tick()` completes immediately; consume it so the
        // first callback fires after the specified delay (browser/Node-like behavior).
        if repeat {
            let mut interval = tokio::time::interval(Duration::from_millis(delay.max(1)));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            interval.tick().await; // consume immediate tick

            while registry_clone.is_timer_active(id) {
                tokio::select! {
                    _ = interval.tick() => {
                        let Some(cb) = registry_clone.get_callback(id) else {
                            break;
                        };
                        if cb.call_async::<_, ()>(None,()).await.is_err() {
                            registry_clone.finish_timer(id);
                            break;
                        }
                    }
                    _ = notifier.notified() => break,
                }
            }

            registry_clone.finish_timer(id);
            return;
        }

        // One-shot timer (setTimeout).
        if delay > 0 {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(delay)) => {}
                _ = notifier.notified() => {
                    return;
                }
            }
        }

        if let Some(cb) = registry_clone.get_callback(id) {
            let _ = cb.call_async::<_, ()>(None, ()).await;
        }
        registry_clone.finish_timer(id);
    });

    id
}

pub fn init(ctx: &JSContext) -> JSResult<()> {
    let registry = {
        let runtime = ctx.runtime();
        runtime.get_or_init_service::<TimerRegistry>().clone()
    };

    let global = ctx.global();

    let registry_clone = registry.clone();
    let set_timeout = JSFunc::new(ctx, move |callback: JSFunc, delay: Optional<f64>| {
        set_timeout_with_repeat(registry_clone.clone(), callback, delay, false)
    });

    let registry_clone = registry.clone();
    let clear_timeout = JSFunc::new(ctx, move |id: JSValue| {
        if let Ok(id) = id.try_into::<u32>() {
            registry_clone.cancel_timer(id);
        }
    });

    let registry_clone = registry.clone();
    let set_interval = JSFunc::new(ctx, move |callback: JSFunc, delay: Optional<f64>| {
        set_timeout_with_repeat(registry_clone.clone(), callback, delay, true)
    });

    let clear_interval = JSFunc::new(ctx, move |id: JSValue| {
        if let Ok(id) = id.try_into::<u32>() {
            registry.cancel_timer(id);
        }
    });

    global.set("setTimeout", set_timeout)?;
    global.set("clearTimeout", clear_timeout)?;
    global.set("setInterval", set_interval)?;
    global.set("clearInterval", clear_interval)?;

    promise::init(ctx)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong_test::*;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicI32, Ordering};
    use tokio::time::sleep;

    #[test]
    fn test_set_interval_without_cancel() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx).unwrap();

            let counter = Rc::new(AtomicI32::new(0));
            let counter_clone = counter.clone();

            let increment = JSFunc::new(&ctx, move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            });
            ctx.global().set("increment", increment)?;

            // Keep the interval handle in scope
            let _interval_id: u32 = ctx
                .eval(Source::from_bytes("setInterval(increment, 50)"))
                .unwrap();

            // Wait for multiple intervals
            sleep(Duration::from_millis(175)).await;
            let count = counter.load(Ordering::SeqCst);
            assert!(
                (3..=5).contains(&count),
                "Expected 3 to 5 increments, got {}",
                count
            );

            // without cancel explicitly, it should no panic!
            Ok(())
        })
    }

    #[test]
    fn test_timer() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx).unwrap();

            rong_console::init(&ctx)?;
            rong_assert::init(&ctx)?;

            let passed = UnitJSRunner::load_script(&ctx, "timer.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        })
    }
}
