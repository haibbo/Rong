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

use rong_js::{function::Optional, JSContext, JSFunc, JSResult, JSRuntimeService};

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use tokio::sync::Notify;
use tokio::time::Duration;

mod promise;

#[derive(Clone)]
pub struct TimerRegistry {
    inner: Rc<TimerRegistryInner>,
}

struct TimerRegistryInner {
    next_id: AtomicU32,
    notifiers: Mutex<HashMap<u32, Rc<Notify>>>,
}

impl JSRuntimeService for TimerRegistry {
    fn on_shutdown(&self) {
        self.shutdown();
    }
}

impl Default for TimerRegistry {
    fn default() -> Self {
        Self {
            inner: Rc::new(TimerRegistryInner {
                next_id: AtomicU32::new(0),
                notifiers: Mutex::new(HashMap::new()),
            }),
        }
    }
}

impl TimerRegistry {
    fn next_id(&self) -> u32 {
        self.inner.next_id.fetch_add(1, Ordering::Relaxed)
    }

    fn register_timer(&self, id: u32, notifier: Rc<Notify>) {
        self.inner.notifiers.lock().unwrap().insert(id, notifier);
    }

    fn cancel_timer(&self, id: u32) {
        if let Some(notifier) = self.inner.notifiers.lock().unwrap().remove(&id) {
            notifier.notify_waiters();
        }
    }

    fn is_timer_active(&self, id: u32) -> bool {
        self.inner.notifiers.lock().unwrap().contains_key(&id)
    }

    fn shutdown(&self) {
        let mut notifiers = self.inner.notifiers.lock().unwrap();
        if notifiers.is_empty() {
            return;
        }

        for (_id, notifier) in notifiers.drain() {
            // println!("Cleaning up timer {}", _id);
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

    let ctx = callback.get_ctx();
    let scheduler_shutdown = ctx.runtime().get_shutdown_signal();

    registry.register_timer(id, notifier.clone());
    let delay = delay.unwrap_or(0.0).max(0.0) as u64;

    ctx.spawn_local(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(delay.max(1)));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        // Initial delay
        if delay > 0 {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(delay)) => {}
                _ = notifier.notified() => return Ok(()),
                _ = scheduler_shutdown.notified() => return Ok(()),
            }
        }

        // First execution
        if registry.is_timer_active(id) && callback.call::<_, ()>(None, ()).is_ok() && !repeat {
            return Ok(());
        }

        // Repeat loop
        while registry.is_timer_active(id) {
            tokio::select! {
                _ = interval.tick() => {
                    if callback.call::<_, ()>(None,()).is_err() {
                        break;
                    }
                }
                _ = notifier.notified() => break,
                _ = scheduler_shutdown.notified() => break,
            }
        }

        Ok(())
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
    let clear_timeout = JSFunc::new(ctx, move |id: u32| {
        registry_clone.cancel_timer(id);
    });

    let registry_clone = registry.clone();
    let set_interval = JSFunc::new(ctx, move |callback: JSFunc, delay: Optional<f64>| {
        set_timeout_with_repeat(registry_clone.clone(), callback, delay, true)
    });

    let clear_interval = JSFunc::new(ctx, move |id: u32| {
        registry.cancel_timer(id);
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

            console::init(&ctx)?;
            assert::init(&ctx)?;

            let passed = UnitJSRunner::load_script(&ctx, "timer.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        })
    }
}
