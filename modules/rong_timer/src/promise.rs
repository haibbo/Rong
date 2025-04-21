use futures::Stream;
use rong::{function::*, *};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::{mpsc, Notify};
use tokio::time::{Interval, interval, sleep};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};

// TODO: support value and TimerOptions for setTimeout and setInterval
// #[derive(FromJSObj)]
// struct TimerOptions {
//     abort: Option<AbortSignal>,
// }

fn get_current_timestamp() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as f64
}

// Promise-based setTimeout - returns a Promise that resolves after the delay
async fn set_timeout(ctx: JSContext, delay: Optional<f64>) -> JSResult<f64> {
    let delay = delay.0.unwrap_or(0.0).max(0.0) as u64;
    
    // Create a notifier for cancellation
    let notifier = Rc::new(Notify::new());
    let notifier_clone = notifier.clone();
    
    // Register with registry (which will clean up on shutdown)
    let registry = ctx.runtime().get_or_init_service::<super::TimerRegistry>().clone();
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
    let notifier = Rc::new(Notify::new());
    let notifier_clone = notifier.clone();
    
    // Register with registry (which will clean up on shutdown)
    let registry = ctx.runtime().get_or_init_service::<super::TimerRegistry>().clone();
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

// This is safe to implement because we don't use Rc directly in the IntervalStream
// The registry is only used in the setup and Drop implementation
unsafe impl Send for IntervalStream {}

impl Stream for IntervalStream {
    type Item = JSResult<f64>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Check if timer was canceled or notification received
        if self.canceled.load(Ordering::SeqCst) || 
           matches!(self.notify_rx.poll_recv(cx), Poll::Ready(_)) {
            return Poll::Ready(None);
        }

        // Then check if interval has ticked
        match self.interval.poll_tick(cx) {
            Poll::Ready(_) => Poll::Ready(Some(Ok(get_current_timestamp()))),
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
    let delay = delay.0.unwrap_or(0.0);
    let delay = if delay < 0.0 { 0.0 } else { delay };

    // Create a channel for cancellation notification
    let (notify_tx, notify_rx) = mpsc::channel::<()>(1);
    
    // Get the timer registry
    let registry = ctx.runtime().get_or_init_service::<super::TimerRegistry>().clone();
    let timer_id = registry.next_id();
    
    // Create a notifier and register it
    let notifier = Rc::new(Notify::new());
    let notifier_clone = notifier.clone();
    registry.register_timer(timer_id, notifier);
    
    // Create a shared cancellation flag
    let canceled = std::sync::Arc::new(AtomicBool::new(false));
    let canceled_clone = canceled.clone();
    
    // Setup a background task that will relay cancellation signal to the channel
    tokio::task::spawn_local(async move {
        notifier_clone.notified().await;
        let _ = notify_tx.send(()).await;
        canceled_clone.store(true, Ordering::SeqCst);
    });

    // Create the stream with an interval that ticks at the specified rate
    let stream = IntervalStream {
        interval: interval(Duration::from_secs_f64(delay / 1000.0)),
        notify_rx,
        registry: registry.clone(),
        timer_id,
        canceled,
    };

    // Convert to JavaScript async iterator
    stream.into_js_async_iter(&ctx)
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    let timer = JSObject::new(ctx);

    timer
        .set(
            "setTimeout",
            JSFunc::new(ctx, set_timeout)?.name("setTimeout")?,
        )?
        .set(
            "setImmediate",
            JSFunc::new(ctx, set_immediate)?.name("setImmediate")?,
        )?
        .set(
            "setInterval",
            JSFunc::new(ctx, set_interval)?.name("setInterval")?,
        )?;

    ctx.global().set("timer", timer)?;
    Ok(())
}
