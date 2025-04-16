use futures::Stream;
use rong_js::{function::*, *};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::{interval, sleep, Interval};

// TODO: support value and TimerOptions for setTimeout and setInterval
// #[derive(FromJSObj)]
// struct TimerOptions {
//     abort: Option<AbortSignal>,
// }

// Promise-based setTimeout - returns a Promise that resolves after the delay
async fn set_timeout(delay: Optional<f64>) -> JSResult<f64> {
    let delay = delay.0.unwrap_or(0.0).max(0.0) as u64;
    sleep(Duration::from_millis(delay)).await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as f64;
    Ok(now)
}

// Promise-based setImmediate - returns a Promise that resolves on next tick
async fn set_immediate() -> JSResult<f64> {
    tokio::task::yield_now().await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as f64;
    Ok(now)
}

// Async iterator for setInterval
struct IntervalStream {
    interval: Interval,
}

impl Stream for IntervalStream {
    type Item = JSResult<f64>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.interval.poll_tick(cx) {
            Poll::Ready(_) => {
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

    let stream = IntervalStream {
        interval: interval(Duration::from_secs_f64(delay / 1000.0)),
    };

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
