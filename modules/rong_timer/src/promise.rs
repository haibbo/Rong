use futures::Stream;
use rong::{function::*, *};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::{Notify, mpsc};
use tokio::time::{Interval, interval, sleep};

// TODO: support value and TimerOptions for setTimeout and setInterval
// #[derive(FromJSObj)]
// struct TimerOptions {
//     abort: Option<AbortSignal>,
// }

fn get_current_timestamp() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

// Promise-based setTimeout - returns a Promise that resolves after the delay
async fn set_timeout(ctx: JSContext, delay: Optional<f64>) -> JSResult<f64> {
    let delay = delay.0.unwrap_or(0.0).max(0.0) as u64;

    // Create a notifier for cancellation
    let notifier = Arc::new(Notify::new());
    let notifier_clone = notifier.clone();

    // Register with registry (which will clean up on shutdown)
    let registry = ctx
        .runtime()
        .get_or_init_service::<super::TimerRegistry>()
        .clone();
    let timer_id = registry.next_id();
    registry.register_timer(timer_id, notifier);

    // Set up the timeout and wait
    let result = tokio::select! {
        _ = sleep(Duration::from_millis(delay)) => Ok(get_current_timestamp()),
        _ = notifier_clone.notified() => Ok(get_current_timestamp()),
    };

    // Unregister from registry
    registry.cancel_timer(timer_id);
    result
}

// Promise-based setImmediate - returns a Promise that resolves on next tick
async fn set_immediate(ctx: JSContext) -> JSResult<f64> {
    // Create a notifier for cancellation
    let notifier = Arc::new(Notify::new());
    let notifier_clone = notifier.clone();

    // Register with registry (which will clean up on shutdown)
    let registry = ctx
        .runtime()
        .get_or_init_service::<super::TimerRegistry>()
        .clone();
    let timer_id = registry.next_id();
    registry.register_timer(timer_id, notifier);

    // Set up the immediate execution and wait
    let result = tokio::select! {
        _ = tokio::task::yield_now() => Ok(get_current_timestamp()),
        _ = notifier_clone.notified() => Ok(get_current_timestamp()),
    };

    // Unregister from registry
    registry.cancel_timer(timer_id);
    result
}

// Async iterator for setInterval - each tick yields current timestamp
struct IntervalStream {
    interval: Interval,
    notify_rx: tokio::sync::mpsc::Receiver<()>,
    registry: super::TimerRegistry, // Keep reference to registry for cleanup
    timer_id: u32,                  // Store timer_id for cleanup
    canceled: std::sync::Arc<AtomicBool>,
}

impl Stream for IntervalStream {
    type Item = f64;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Check if timer was canceled or notification received
        if self.canceled.load(Ordering::SeqCst)
            || matches!(self.notify_rx.poll_recv(cx), Poll::Ready(_))
        {
            return Poll::Ready(None);
        }

        // Then check if interval has ticked
        match self.interval.poll_tick(cx) {
            Poll::Ready(_) => Poll::Ready(Some(get_current_timestamp())),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for IntervalStream {
    fn drop(&mut self) {
        // Mark as canceled so any remaining callbacks know to stop
        self.canceled.store(true, Ordering::SeqCst);

        // Explicitly cancel the timer in the registry to decrement active count
        self.registry.cancel_timer(self.timer_id);
    }
}

// Promise-based setInterval - returns an async iterator that yields timestamps
pub fn set_interval(ctx: JSContext, delay: Optional<f64>) -> JSResult<JSObject> {
    let delay_ms = delay.0.unwrap_or(0.0);
    let delay_ms = if delay_ms.is_finite() && delay_ms > 0.0 {
        delay_ms
    } else {
        0.0
    };

    // Create a channel for cancellation notification
    let (notify_tx, notify_rx) = mpsc::channel::<()>(1);

    // Get the timer registry
    let registry = ctx
        .runtime()
        .get_or_init_service::<super::TimerRegistry>()
        .clone();
    let timer_id = registry.next_id();

    // Create a notifier and register it
    let notifier = Arc::new(Notify::new());
    let notifier_clone = notifier.clone();
    registry.register_timer(timer_id, notifier);

    // Create a shared cancellation flag
    let canceled = std::sync::Arc::new(AtomicBool::new(false));
    let canceled_clone = canceled.clone();

    // Setup a background task that will relay cancellation signal to the channel
    spawn_local(async move {
        notifier_clone.notified().await;
        let _ = notify_tx.send(()).await;
        canceled_clone.store(true, Ordering::SeqCst);
    });

    // Create the stream with an interval that ticks at the specified rate
    let stream = IntervalStream {
        interval: interval(Duration::from_millis(delay_ms as u64).max(Duration::from_millis(1))),
        notify_rx,
        registry: registry.clone(),
        timer_id,
        canceled,
    };

    // Convert to JavaScript async iterator
    stream.to_js_async_iter(&ctx)
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    let timer = JSObject::new(ctx);

    timer.set(
        "setTimeout",
        JSFunc::new(ctx, set_timeout)?.name("setTimeout")?,
    )?;
    timer.set(
        "setImmediate",
        JSFunc::new(ctx, set_immediate)?.name("setImmediate")?,
    )?;
    timer.set(
        "setInterval",
        JSFunc::new(ctx, set_interval)?.name("setInterval")?,
    )?;

    ctx.global().set("timers", timer)?;
    Ok(())
}
