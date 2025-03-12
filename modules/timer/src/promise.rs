use futures::Stream;
use rusty_js::{function::*, *};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::time::{interval, sleep, Interval};

// Promise-based setTimeout
async fn set_timeout(_ctx: JSContext, callback: JSFunc, delay: Optional<f64>) {
    let delay = delay.0.unwrap_or(0.0).max(0.0) as u64;
    sleep(Duration::from_millis(delay)).await;
    let _ = callback.call::<_, ()>(None, ());
}

// Promise-based setImmediate
async fn set_immediate(_ctx: JSContext, callback: JSFunc) {
    tokio::task::yield_now().await;
    let _ = callback.call::<_, ()>(None, ());
}

// ToJSAsyncIterator need IntervalStream Sendable, but JSFunc does not support
// so we box it
struct IntervalStream {
    callback: usize,
    interval: Interval,
}

impl Stream for IntervalStream {
    type Item = ();

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.interval.poll_tick(cx) {
            Poll::Ready(_) => {
                let callback = self.callback as *mut JSFunc;
                let callback = unsafe { (*callback).clone() };
                let _ = callback.call::<_, ()>(None, ());
                Poll::Ready(Some(()))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Drop for IntervalStream {
    fn drop(&mut self) {
        let _ = unsafe { Box::from_raw(self.callback as *mut JSFunc) };
    }
}

// Promise-based setInterval that returns an async iterator
pub fn set_interval(ctx: JSContext, callback: JSFunc, delay: Optional<f64>) -> JSResult<JSObject> {
    let delay = delay.0.unwrap_or(0.0);
    let delay = if delay < 0.0 { 0.0 } else { delay };

    let stream = IntervalStream {
        callback: Box::into_raw(Box::new(callback)) as usize,
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
