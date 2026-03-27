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

use rong::{
    JSContext, JSFunc, JSResult, JSRuntimeService, JSValue, RongExecutor, function::Optional,
    spawn_local,
};

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard};
use tokio::sync::Notify;
use tokio::sync::mpsc;
use tokio::time::Duration;

mod promise;

fn lock_poison<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

struct TimerCallbackQueue {
    tx: mpsc::UnboundedSender<u32>,
    rx: RefCell<Option<mpsc::UnboundedReceiver<u32>>>,
}

impl Default for TimerCallbackQueue {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            rx: RefCell::new(Some(rx)),
        }
    }
}

impl JSRuntimeService for TimerCallbackQueue {
    fn on_shutdown(&self) {
        // Drop receiver so background senders start failing fast.
        let _ = self.rx.borrow_mut().take();
    }
}

impl TimerCallbackQueue {
    fn tx(&self) -> mpsc::UnboundedSender<u32> {
        self.tx.clone()
    }

    fn start(&self, registry: TimerRegistry) {
        let Some(mut rx) = self.rx.borrow_mut().take() else {
            return;
        };

        spawn_local(async move {
            while let Some(id) = rx.recv().await {
                let (callback, repeat, pending) = {
                    let mut timers = lock_poison(&registry.inner.timers);
                    let Some(entry) = timers.get_mut(&id) else {
                        continue;
                    };
                    let callback = entry.callback.clone();
                    let repeat = entry.repeat;
                    let pending = entry.pending.clone();
                    if !repeat {
                        // One-shot: drop callback on the JS thread after this dispatch.
                        let _ = timers.remove(&id);
                    }
                    (callback, repeat, pending)
                };

                if let Some(cb) = callback {
                    // Callback-style timers should not await returned promises; just invoke.
                    let result = cb.call::<_, JSValue>(None, ());
                    if result.is_err() && repeat {
                        // Best-effort: stop interval on unhandled exception.
                        registry.cancel_timer(id);
                    }
                }

                // Mark one queued tick as processed.
                let _ = pending.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                    Some(v.saturating_sub(1))
                });
            }
        });
    }
}

#[derive(Clone)]
pub struct TimerRegistry {
    inner: Rc<TimerRegistryInner>,
}

struct TimerEntry {
    cancel: Arc<Notify>,
    // For callback-style timers (global setTimeout/setInterval).
    // We keep the JSFunc in the registry so `on_shutdown` can drop it before the JS runtime is freed,
    // even if the spawned timer task hasn't been polled/canceled yet.
    callback: Option<JSFunc>,
    repeat: bool,
    pending: Arc<AtomicU8>,
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

    fn register_timer(&self, id: u32, cancel: Arc<Notify>) {
        lock_poison(&self.inner.timers).insert(
            id,
            TimerEntry {
                cancel,
                callback: None,
                repeat: false,
                pending: Arc::new(AtomicU8::new(0)),
            },
        );
    }

    fn register_timer_with_callback(
        &self,
        id: u32,
        cancel: Arc<Notify>,
        pending: Arc<AtomicU8>,
        callback: JSFunc,
        repeat: bool,
    ) {
        lock_poison(&self.inner.timers).insert(
            id,
            TimerEntry {
                cancel,
                callback: Some(callback),
                repeat,
                pending,
            },
        );
    }

    fn cancel_timer(&self, id: u32) {
        if let Some(entry) = lock_poison(&self.inner.timers).remove(&id) {
            entry.cancel.notify_waiters();
        }
    }

    fn get_entry_for_bg(&self, id: u32) -> Option<(Arc<Notify>, Arc<AtomicU8>)> {
        let timers = lock_poison(&self.inner.timers);
        let entry = timers.get(&id)?;
        Some((entry.cancel.clone(), entry.pending.clone()))
    }

    fn shutdown(&self) {
        let mut timers = lock_poison(&self.inner.timers);
        if timers.is_empty() {
            return;
        }

        // Copy the notifiers before draining to avoid deadlock
        let notifiers_copy: Vec<Arc<Notify>> = timers.values().map(|e| e.cancel.clone()).collect();
        timers.clear(); // drops callbacks before the JS runtime is freed

        // Notify all timers outside the lock
        for notifier in notifiers_copy {
            notifier.notify_waiters();
        }
    }
}

fn set_timeout_with_repeat(
    registry: TimerRegistry,
    callback_tx: mpsc::UnboundedSender<u32>,
    callback: JSFunc,
    delay: Optional<f64>,
    repeat: bool,
) -> u32 {
    const MAX_QUEUED_TICKS: u8 = 8;

    let id = registry.next_id();
    let cancel = Arc::new(Notify::new());
    let pending = Arc::new(AtomicU8::new(0));

    registry.register_timer_with_callback(id, cancel.clone(), pending.clone(), callback, repeat);
    let delay = delay.unwrap_or(0.0).max(0.0) as u64;
    let interval_duration = Duration::from_millis(delay.max(1));

    // Grab the per-timer cancel/pending handles for cross-thread operation.
    let Some((cancel_bg, pending_bg)) = registry.get_entry_for_bg(id) else {
        return id;
    };

    let run_timer = move |cancel: Arc<Notify>,
                          pending: Arc<AtomicU8>,
                          tx: mpsc::UnboundedSender<u32>| async move {
        let send_tick = || {
            // Keep a small bounded backlog so intervals can "catch up" after brief stalls
            // without letting unbounded channels grow forever.
            let mut cur = pending.load(Ordering::SeqCst);
            loop {
                if cur >= MAX_QUEUED_TICKS {
                    return;
                }
                match pending.compare_exchange(
                    cur,
                    cur.saturating_add(1),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => break,
                    Err(next) => cur = next,
                }
            }
            if tx.send(id).is_err() {
                // Receiver dropped: balance the queued count so future timers don't stall.
                let _ = pending.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                    Some(v.saturating_sub(1))
                });
            }
        };

        if repeat {
            let mut next_deadline = tokio::time::Instant::now() + interval_duration;
            loop {
                tokio::select! {
                    _ = tokio::time::sleep_until(next_deadline) => {}
                    _ = cancel.notified() => break,
                }
                send_tick();

                next_deadline += interval_duration;
                let now = tokio::time::Instant::now();
                if next_deadline <= now {
                    next_deadline = now + interval_duration;
                }
            }
            return;
        }

        if delay > 0 {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(delay)) => {}
                _ = cancel.notified() => return,
            }
        }
        send_tick();
    };

    // Run timer on the global host executor for reliable timing.
    RongExecutor::global().spawn(run_timer(cancel_bg, pending_bg, callback_tx));

    id
}

pub fn init(ctx: &JSContext) -> JSResult<()> {
    let registry = {
        let runtime = ctx.runtime();
        runtime.get_or_init_service::<TimerRegistry>().clone()
    };

    let callback_queue = ctx.runtime().get_or_init_service::<TimerCallbackQueue>();
    callback_queue.start(registry.clone());
    let callback_tx = callback_queue.tx();

    let global = ctx.global();

    let registry_clone = registry.clone();
    let callback_tx_clone = callback_tx.clone();
    let set_timeout = JSFunc::new(ctx, move |callback: JSFunc, delay: Optional<f64>| {
        set_timeout_with_repeat(
            registry_clone.clone(),
            callback_tx_clone.clone(),
            callback,
            delay,
            false,
        )
    });

    let registry_clone = registry.clone();
    let clear_timeout = JSFunc::new(ctx, move |id: JSValue| {
        if let Ok(id) = id.try_into::<u32>() {
            registry_clone.cancel_timer(id);
        }
    });

    let registry_clone = registry.clone();
    let callback_tx_clone = callback_tx.clone();
    let set_interval = JSFunc::new(ctx, move |callback: JSFunc, delay: Optional<f64>| {
        set_timeout_with_repeat(
            registry_clone.clone(),
            callback_tx_clone.clone(),
            callback,
            delay,
            true,
        )
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
