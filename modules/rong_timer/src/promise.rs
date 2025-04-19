use futures::Stream;
use rong::{function::*, *};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{Interval, interval, sleep};

// TODO: support value and TimerOptions for setTimeout and setInterval
// #[derive(FromJSObj)]
// struct TimerOptions {
//     abort: Option<AbortSignal>,
// }

// Promise-based setTimeout - returns a Promise that resolves after the delay
async fn set_timeout(ctx: JSContext, delay: Optional<f64>) -> JSResult<f64> {
    let delay = delay.0.unwrap_or(0.0).max(0.0) as u64;
    let shutdown = ctx.runtime().get_shutdown_signal();

    tokio::select! {
        _ = sleep(Duration::from_millis(delay)) => {}
        _ = shutdown.notified() => {}
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as f64;
    Ok(now)
}

// Promise-based setImmediate - returns a Promise that resolves on next tick
async fn set_immediate(ctx: JSContext) -> JSResult<f64> {
    let shutdown = ctx.runtime().get_shutdown_signal();

    tokio::select! {
        _ = tokio::task::yield_now() => {}
        _ = shutdown.notified() => {}
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as f64;
    Ok(now)
}

// Async iterator for setInterval - each tick yields current timestamp
// Uses a separate channel for shutdown signal so the stream can be Send
struct IntervalStream {
    interval: Interval,
    shutdown_rx: mpsc::Receiver<()>,
}

impl Stream for IntervalStream {
    type Item = JSResult<f64>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // First check if shutdown signal was received
        match self.shutdown_rx.poll_recv(cx) {
            Poll::Ready(_) => return Poll::Ready(None), // End stream on shutdown
            Poll::Pending => (),                        // Continue if no shutdown signal
        }

        // Then check if interval has ticked
        match self.interval.poll_tick(cx) {
            Poll::Ready(_) => {
                // Yield current timestamp
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as f64;
                Poll::Ready(Some(Ok(now)))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

// Promise-based setInterval - returns an async iterator that yields timestamps
pub fn set_interval(ctx: JSContext, delay: Optional<f64>) -> JSResult<JSObject> {
    let delay = delay.0.unwrap_or(0.0);
    let delay = if delay < 0.0 { 0.0 } else { delay };

    // Create a channel for shutdown notification
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);

    // Setup a background task that will signal shutdown
    let shutdown = ctx.runtime().get_shutdown_signal();
    tokio::task::spawn_local(async move {
        shutdown.notified().await;
        let _ = shutdown_tx.send(()).await;
        // Channel will be closed when task ends
    });

    // Create the stream with an interval that ticks at the specified rate
    let stream = IntervalStream {
        interval: interval(Duration::from_secs_f64(delay / 1000.0)),
        shutdown_rx,
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
