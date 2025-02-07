//! Timer implementation for rusty-js
//!
//! This module provides timer functionality similar to Web APIs:
//! - setTimeout/clearTimeout
//! - setInterval/clearInterval
//!
//! # Limitations
//! - Unlike Web APIs, this implementation does not support passing additional arguments
//!   to the callback function. Only the callback function and delay are supported.
//! - Delay is in milliseconds and should be a positive number.
//!
//! # Example
//! ```rust,no_run
//! use rusty_js::*;
//! use rustyjs_test::*;
//! use timer::init;
//! use tokio::time::{Duration,sleep};
//!
//! async_run!(|ctx: JSContext| async move {
//!
//! init(&ctx).unwrap();
//!
//! ctx.global().set(
//!     "print",
//!     JSFunc::new(&ctx, |msg: String| println!("{}", msg))
//! );
//!
//! ctx.eval::<()>(Source::from_bytes(r#"
//!     setTimeout(() => print('Timeout!'), 1000);
//!     setInterval(() => print('Interval!'), 1000);
//! "#)).unwrap();
//!
//! sleep(Duration::from_millis(2500)).await;
//! Ok(())
//! });
//! ```

use rusty_js::{function::Optional, JSContext, JSFunc, JSResult};

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::Notify;

static NEXT_ID: AtomicU32 = AtomicU32::new(0);
static CANCEL_NOTIFIERS: LazyLock<Mutex<HashMap<u32, Arc<Notify>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn set_timeout_with_repeat(
    ctx: &JSContext,
    callback: JSFunc,
    delay: Optional<f64>,
    repeat: bool,
) -> u32 {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let notifier = Arc::new(Notify::new());

    CANCEL_NOTIFIERS
        .lock()
        .unwrap()
        .insert(id, notifier.clone());

    // Convert negative delay to 0, following browser behavior
    let delay = delay.unwrap_or(0.0).max(0.0) as u64;

    ctx.spawn_local(async move {
        // Use a fixed minimum interval of 1ms since tokio::interval doesn't support zero intervals
        let mut interval = tokio::time::interval(Duration::from_millis(1));

        // For non-zero delays, wait before first execution
        if delay > 0 {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_millis(delay)) => {}
                _ = notifier.notified() => {
                    CANCEL_NOTIFIERS.lock().unwrap().remove(&id);
                    return Ok(());
                }
            }
        }

        // Check if timer was cancelled during initial delay
        if !CANCEL_NOTIFIERS.lock().unwrap().contains_key(&id) {
            return Ok(());
        }

        // Execute first callback
        if callback.call::<_, ()>(()).is_ok() {
            // For setTimeout, exit after first execution
            if !repeat {
                CANCEL_NOTIFIERS.lock().unwrap().remove(&id);
                return Ok(());
            }
        } else {
            // Clean up and exit if callback fails
            CANCEL_NOTIFIERS.lock().unwrap().remove(&id);
            return Ok(());
        }

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Check if timer was cancelled
                    if !CANCEL_NOTIFIERS.lock().unwrap().contains_key(&id) {
                        break;
                    }

                    // Execute callback and break loop if it fails
                    if callback.call::<_, ()>(()).is_err() {
                        break;
                    }
                }
                _ = notifier.notified() => {
                    // Timer was cancelled
                    break;
                }
            }
        }

        // Clean up resources
        CANCEL_NOTIFIERS.lock().unwrap().remove(&id);
        Ok(())
    });

    id
}

fn cancel_timeout(id: u32) {
    if let Some(notifier) = CANCEL_NOTIFIERS.lock().unwrap().remove(&id) {
        notifier.notify_waiters();
    }
}

pub fn init(ctx: &JSContext) -> JSResult<()> {
    let global = ctx.global();

    let set_timeout = JSFunc::new(
        ctx,
        |ctx: &JSContext, callback: JSFunc, delay: Optional<f64>| {
            set_timeout_with_repeat(ctx, callback, delay, false)
        },
    );

    let clear_timeout = JSFunc::new(ctx, |id: u32| {
        cancel_timeout(id);
    });

    let set_interval = JSFunc::new(
        ctx,
        |ctx: &JSContext, callback: JSFunc, delay: Optional<f64>| {
            set_timeout_with_repeat(ctx, callback, delay, true)
        },
    );

    let clear_interval = JSFunc::new(ctx, |id: u32| {
        cancel_timeout(id);
    });

    global.set("setTimeout", set_timeout);
    global.set("clearTimeout", clear_timeout);
    global.set("setInterval", set_interval);
    global.set("clearInterval", clear_interval);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyjs_test::*;
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::sync::Arc;
    use tokio::time::sleep;

    #[test]
    fn test_set_timeout() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx).unwrap();

            let result: i32 = ctx
                .eval::<Promise>(Source::from_bytes(
                    r#"
                new Promise((resolve) => {
                    setTimeout(() => {
                        resolve(42);
                    }, 100);
                })"#,
                ))
                .unwrap()
                .into_future::<i32>()
                .await
                .unwrap();

            assert_eq!(result, 42);
            Ok(())
        })
    }

    #[test]
    fn test_clear_timeout() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx).unwrap();

            let counter = Arc::new(AtomicI32::new(0));
            let counter_clone = counter.clone();

            let increment = JSFunc::new(&ctx, move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            });
            ctx.global().set("increment", increment);

            ctx.eval::<()>(Source::from_bytes(
                r#"
                let id = setTimeout(increment, 100);
                clearTimeout(id);
            "#,
            ))
            .unwrap();

            // Wait longer than the timeout
            sleep(Duration::from_millis(200)).await;
            assert_eq!(counter.load(Ordering::SeqCst), 0);

            Ok(())
        })
    }

    #[test]
    fn test_set_interval() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx).unwrap();

            let counter = Arc::new(AtomicI32::new(0));
            let counter_clone = counter.clone();

            let increment = JSFunc::new(&ctx, move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            });
            ctx.global().set("increment", increment);

            // Keep the interval handle in scope
            let _interval_id: u32 = ctx
                .eval(Source::from_bytes(
                    r#"
                setInterval(increment, 50)
            "#,
                ))
                .unwrap();

            // Wait for multiple intervals
            sleep(Duration::from_millis(175)).await;
            let count = counter.load(Ordering::SeqCst);
            assert!(count >= 3, "Expected at least 3 increments, got {}", count);

            // cleanup
            cancel_timeout(_interval_id);
            sleep(Duration::from_millis(100)).await;

            Ok(())
        })
    }

    #[test]
    fn test_clear_interval() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx).unwrap();

            let counter = Arc::new(AtomicI32::new(0));
            let counter_clone = counter.clone();

            let increment = JSFunc::new(&ctx, move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            });
            ctx.global().set("increment", increment);

            // Use JavaScript APIs to set and clear interval
            ctx.eval::<()>(Source::from_bytes(
                r#"
                let id = setInterval(increment, 50);
                setTimeout(() => {
                    clearInterval(id);
                }, 125);
            "#,
            ))
            .unwrap();

            // Wait for interval to be cleared
            sleep(Duration::from_millis(150)).await;

            let count = counter.load(Ordering::SeqCst);
            assert!(count >= 2, "Expected at least 2 increments, got {}", count);

            // Wait to ensure no more increments occur
            sleep(Duration::from_millis(100)).await;
            let final_count = counter.load(Ordering::SeqCst);
            assert_eq!(
                count, final_count,
                "Counter should not increase after clearInterval"
            );

            Ok(())
        })
    }

    #[test]
    fn test_timer_edge_cases() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx).unwrap();

            // Test negative delay (should be treated as 0)
            let result: bool = ctx
                .eval::<Promise>(Source::from_bytes(
                    r#"
                new Promise((resolve) => {
                    setTimeout(() => resolve(true), -100);
                })"#,
                ))
                .unwrap()
                .into_future::<bool>()
                .await
                .unwrap();

            assert!(result);

            // Test clearing non-existent timer (should not crash)
            ctx.eval::<()>(Source::from_bytes(
                r#"
                clearTimeout(999999);
                clearInterval(999999);
            "#,
            ))
            .unwrap();

            Ok(())
        })
    }
}
