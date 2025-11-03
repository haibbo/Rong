use crate::writable::WritableStream;
use bytes::Bytes;
use rong::function::This;
use rong::*;
use rong_abort::AbortSignal;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::io::AsyncRead;
use tokio::sync::{Mutex, mpsc};

#[js_export]
pub struct ReadableStream {
    // A single-consumer source guarded by a lock; getReader() takes ownership.
    pub(crate) rx_slot: Arc<StdMutex<Option<mpsc::Receiver<Result<Bytes, String>>>>>,
}

#[js_export]
pub struct ReadableStreamDefaultReader {
    // Reference to the owning stream's slot so releaseLock can return ownership.
    slot: Arc<StdMutex<Option<mpsc::Receiver<Result<Bytes, String>>>>>,
    // Receiver owned by the reader while locked.
    rx: Arc<Mutex<Option<mpsc::Receiver<Result<Bytes, String>>>>>,
}

#[js_export]
pub struct ReadableStreamDefaultController {
    tx: Arc<Mutex<Option<mpsc::Sender<Result<Bytes, String>>>>>,
}

impl ReadableStream {
    pub fn from_receiver(rx: mpsc::Receiver<Result<Bytes, String>>) -> Self {
        Self {
            rx_slot: Arc::new(StdMutex::new(Some(rx))),
        }
    }

    pub fn from_async_reader<R>(mut reader: R, chunk_size: usize) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
    {
        let (tx, rx) = mpsc::channel::<Result<Bytes, String>>(16);
        rong::spawn(async move {
            let mut buf = vec![0u8; chunk_size.max(1)];
            loop {
                match tokio::io::AsyncReadExt::read(&mut reader, &mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx
                            .send(Ok(Bytes::copy_from_slice(&buf[..n])))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string())).await;
                        break;
                    }
                }
            }
        });
        Self::from_receiver(rx)
    }
}

// Options for pipeTo
#[derive(FromJSObj, Default)]
struct PipeToOptions {
    #[rename = "preventClose"]
    prevent_close: Option<bool>,
    #[rename = "preventAbort"]
    prevent_abort: Option<bool>,
    #[rename = "preventCancel"]
    prevent_cancel: Option<bool>,
    signal: Option<AbortSignal>,
}

#[js_class]
impl ReadableStream {
    #[js_method(constructor)]
    fn new(ctx: JSContext, underlying: function::Optional<JSValue>) -> JSResult<JSObject> {
        // Create a basic channel-backed stream
        let (tx, rx) = mpsc::channel::<Result<Bytes, String>>(16);
        let stream = Self::from_receiver(rx);
        let obj = JSReadableStream::new(&ctx, stream)?.into_object();

        // If an underlying source is provided, call start(controller) if present
        if let Some(v) = underlying.0 {
            if let Some(src) = v.into_object() {
                let controller = ReadableStreamDefaultController {
                    tx: Arc::new(Mutex::new(Some(tx))),
                };
                if let Ok(start) = src.get::<_, JSFunc>("start") {
                    let _ = start.call::<_, JSValue>(Some(src.clone()), (controller,));
                }
            }
        }
        Ok(obj)
    }

    #[js_method(rename = "getReader")]
    fn get_reader(&self) -> JSResult<ReadableStreamDefaultReader> {
        let mut guard = self
            .rx_slot
            .lock()
            .map_err(|_| RongJSError::Error("Stream is poisoned".to_string()))?;
        match guard.take() {
            Some(rx) => Ok(ReadableStreamDefaultReader {
                slot: self.rx_slot.clone(),
                rx: Arc::new(Mutex::new(Some(rx))),
            }),
            None => Err(RongJSError::TypeError(
                "ReadableStream is locked".to_string(),
            )),
        }
    }

    #[js_method]
    fn cancel(&self) -> JSResult<()> {
        let mut guard = self
            .rx_slot
            .lock()
            .map_err(|_| RongJSError::Error("Stream is poisoned".to_string()))?;
        *guard = None;
        Ok(())
    }

    /// Minimal pipeTo implementation: pumps from this ReadableStream into a WritableStream.
    /// Honors preventClose/preventAbort/preventCancel. Basic signal support (pre-check).
    #[js_method(rename = "pipeTo")]
    async fn pipe_to(
        &self,
        ctx: JSContext,
        dest: WritableStream,
        options: function::Optional<PipeToOptions>,
    ) -> JSResult<()> {
        let opts = options.0.unwrap_or_default();
        let prevent_close = opts.prevent_close.unwrap_or(false);
        let prevent_abort = opts.prevent_abort.unwrap_or(false);
        let prevent_cancel = opts.prevent_cancel.unwrap_or(false);

        if let Some(sig) = &opts.signal {
            if sig.aborted() {
                return Err(RongJSError::from_jsvalue(sig.get_reason()));
            }
        }

        // Acquire reader and writer
        let mut reader = self.get_reader()?;
        let mut writer = dest.get_writer()?;

        let mut pipe_err: Option<RongJSError> = None;
        let mut abort_rx = opts.signal.as_ref().map(|s| s.subscribe());

        loop {
            // Race read vs abort (if provided)
            let res_obj = if let Some(rx) = &mut abort_rx {
                tokio::select! {
                    r = reader.read(ctx.clone()) => {
                        Some(r?)
                    }
                    reason = rx.recv() => {
                        pipe_err = Some(RongJSError::from_jsvalue(reason));
                        None
                    }
                }
            } else {
                Some(reader.read(ctx.clone()).await?)
            };

            if let Some(r) = res_obj {
                let done = r.get::<_, bool>("done")?;
                if done {
                    break;
                }
                let chunk = r.get::<_, JSValue>("value")?;
                // Also honor abort between read and write
                if let Some(sig) = &opts.signal {
                    if sig.aborted() {
                        pipe_err = Some(RongJSError::from_jsvalue(sig.get_reason()));
                        break;
                    }
                }
                if let Err(e) = writer.write(chunk).await {
                    // Writer error
                    if !prevent_cancel {
                        let _ = reader.cancel().await;
                    }
                    pipe_err = Some(e);
                    break;
                }
                continue;
            } else {
                // Aborted
                break;
            }
        }

        // Close writer on normal completion
        if pipe_err.is_none() && !prevent_close {
            let _ = writer.close().await;
        }

        // Release locks (ignore errors)
        let _ = reader.release_lock().await;
        let _ = writer.release_lock().await;

        // Abort writer on error if needed
        if let Some(e) = pipe_err {
            if !prevent_abort {
                let _ = writer.abort().await;
            }
            return Err(e);
        }
        Ok(())
    }

    /// Split this ReadableStream into two identical branches.
    /// The original stream becomes locked/disturbed; consumers must use the returned branches.
    #[js_method]
    fn tee(&self, ctx: JSContext) -> JSResult<JSArray> {
        // Take ownership of the underlying receiver. If none, the stream is locked.
        let rx = match readable_stream_take_receiver(self) {
            Some(rx) => rx,
            None => {
                return Err(RongJSError::TypeError(
                    "ReadableStream is locked".to_string(),
                ));
            }
        };

        // Create two branches and a task to fan out chunks
        let (tx1, rx1) = mpsc::channel::<Result<Bytes, String>>(16);
        let (tx2, rx2) = mpsc::channel::<Result<Bytes, String>>(16);

        rong::spawn(async move {
            let mut src = rx;
            let mut tx1 = Some(tx1);
            let mut tx2 = Some(tx2);
            while let Some(item) = src.recv().await {
                match item {
                    Ok(bytes) => {
                        // Clone bytes for each branch (Bytes is cheap to clone)
                        if let Some(t1) = tx1.as_mut() {
                            if t1.send(Ok(bytes.clone())).await.is_err() {
                                tx1 = None;
                            }
                        }
                        if let Some(t2) = tx2.as_mut() {
                            if t2.send(Ok(bytes)).await.is_err() {
                                tx2 = None;
                            }
                        }
                        if tx1.is_none() && tx2.is_none() {
                            break;
                        }
                    }
                    Err(e) => {
                        if let Some(t1) = tx1.as_mut() {
                            let _ = t1.send(Err(e.clone())).await;
                        }
                        if let Some(t2) = tx2.as_mut() {
                            let _ = t2.send(Err(e)).await;
                        }
                        break;
                    }
                }
            }
            // Dropping tx1/tx2 closes the branches
        });

        // Wrap the two receivers as ReadableStream instances
        let b1 = JSReadableStream::from_receiver(&ctx, rx1)?.into_object();
        let b2 = JSReadableStream::from_receiver(&ctx, rx2)?.into_object();

        let arr = JSArray::new(&ctx)?;
        arr.set(0, b1)?;
        arr.set(1, b2)?;
        Ok(arr)
    }
}

#[js_class]
impl ReadableStreamDefaultReader {
    #[js_method(constructor)]
    fn new() -> JSResult<Self> {
        Err(RongJSError::TypeError("Illegal constructor".to_string()))
    }

    #[js_method]
    async fn read(&mut self, ctx: JSContext) -> JSResult<JSObject> {
        // Take the receiver out to avoid holding the lock across await
        let mut rx_opt = {
            let mut slot = self.rx.lock().await;
            slot.take()
        };

        // If already released or stream canceled
        let mut rx = match rx_opt.take() {
            Some(rx) => rx,
            None => {
                let out = JSObject::new(&ctx);
                out.set("done", true)?;
                return Ok(out);
            }
        };

        let next = rx.recv().await;
        // Put the receiver back
        {
            let mut slot = self.rx.lock().await;
            *slot = Some(rx);
        }

        match next {
            Some(Ok(bytes)) => {
                let out = JSObject::new(&ctx);
                out.set("done", false)?;
                let ab = JSArrayBuffer::<u8>::from_bytes(&ctx, &bytes)?;
                out.set("value", ab)?;
                Ok(out)
            }
            Some(Err(e)) => Err(RongJSError::Error(e)),
            None => {
                // closed
                let out = JSObject::new(&ctx);
                out.set("done", true)?;
                Ok(out)
            }
        }
    }

    #[js_method(rename = "releaseLock")]
    async fn release_lock(&mut self) -> JSResult<()> {
        // Take receiver out
        let rx_opt = {
            let mut slot = self.rx.lock().await;
            slot.take()
        };
        // Return it back to the stream's slot so another reader can be acquired
        if let Some(rx) = rx_opt {
            let mut guard = self
                .slot
                .lock()
                .map_err(|_| RongJSError::Error("Stream is poisoned".to_string()))?;
            if guard.is_none() {
                *guard = Some(rx);
            }
        }
        Ok(())
    }

    #[js_method]
    async fn cancel(&mut self) -> JSResult<()> {
        let mut slot = self.rx.lock().await;
        *slot = None;
        Ok(())
    }
}

#[js_class]
impl ReadableStreamDefaultController {
    #[js_method(constructor)]
    fn new() -> JSResult<Self> {
        Err(RongJSError::TypeError("Illegal constructor".to_string()))
    }

    #[js_method]
    fn enqueue(&mut self, chunk: JSValue) -> JSResult<()> {
        // Support Uint8Array or ArrayBuffer
        let bytes: Bytes = if let Some(obj) = chunk.clone().into_object() {
            if let Some(ta) = JSTypedArray::from_object(obj.clone()) {
                if let Some(b) = ta.as_bytes() {
                    Bytes::copy_from_slice(b)
                } else {
                    return Err(RongJSError::TypeError("Invalid TypedArray".to_string()));
                }
            } else if let Some(ab) = JSArrayBuffer::<u8>::from_object(obj) {
                if let Some(b) = ab.as_bytes() {
                    Bytes::copy_from_slice(b)
                } else {
                    return Err(RongJSError::TypeError("Invalid ArrayBuffer".to_string()));
                }
            } else {
                return Err(RongJSError::TypeError(
                    "enqueue expects Uint8Array or ArrayBuffer".to_string(),
                ));
            }
        } else {
            return Err(RongJSError::TypeError(
                "enqueue expects a TypedArray or ArrayBuffer".to_string(),
            ));
        };

        let mut slot = futures::executor::block_on(self.tx.lock());
        if let Some(tx) = slot.as_mut() {
            futures::executor::block_on(async {
                let _ = tx.send(Ok(bytes)).await;
            });
            Ok(())
        } else {
            Err(RongJSError::Error("ReadableStream is closed".to_string()))
        }
    }

    #[js_method]
    fn close(&mut self) -> JSResult<()> {
        let mut slot = futures::executor::block_on(self.tx.lock());
        *slot = None;
        Ok(())
    }

    #[js_method]
    fn error(&mut self, reason: function::Optional<JSValue>) -> JSResult<()> {
        let msg = reason
            .0
            .map(|v| v.to_string())
            .unwrap_or_else(|| "ReadableStream error".to_string());
        let mut slot = futures::executor::block_on(self.tx.lock());
        if let Some(tx) = slot.as_mut() {
            let _ = futures::executor::block_on(async { tx.send(Err(msg)).await });
        }
        *slot = None;
        Ok(())
    }
}

/// Take the underlying receiver from a ReadableStream. This consumes the stream's
/// readable side and returns the channel for Rust consumers.
pub fn readable_stream_take_receiver(
    rs: &ReadableStream,
) -> Option<mpsc::Receiver<Result<Bytes, String>>> {
    let mut guard = rs.rx_slot.lock().ok()?;
    guard.take()
}

/// Check if a ReadableStream is locked (a reader has been acquired)
pub fn readable_stream_is_locked(rs: &ReadableStream) -> bool {
    rs.rx_slot.lock().map(|g| g.is_none()).unwrap_or(true)
}

// Install instance-level async iterator: stream implements next() and [Symbol.asyncIterator]
fn install_instance_async_iter(ctx: &JSContext, obj: &JSObject) -> JSResult<()> {
    // next() method on the instance (use `this` to borrow the underlying Rust object)
    let next_fn = JSFunc::new(
        ctx,
        move |ctx: JSContext, this: This<JSObject>| async move {
            if let Ok(rs) = (*this).borrow::<ReadableStream>() {
                if let Some(mut rx) = readable_stream_take_receiver(&*rs) {
                    let item = rx.recv().await;
                    match item {
                        Some(Ok(bytes)) => {
                            if let Ok(mut slot) = rs.rx_slot.lock() {
                                if slot.is_none() {
                                    *slot = Some(rx);
                                }
                            }
                            let out = JSObject::new(&ctx);
                            out.set("done", false).ok();
                            if let Ok(ab) = JSArrayBuffer::<u8>::from_bytes(&ctx, &bytes) {
                                out.set("value", ab).ok();
                            }
                            Ok(out)
                        }
                        Some(Err(e)) => Err(RongJSError::Error(e)),
                        None => {
                            let out = JSObject::new(&ctx);
                            out.set("done", true).ok();
                            out.set("value", JSValue::undefined(&ctx)).ok();
                            Ok(out)
                        }
                    }
                } else {
                    let out = JSObject::new(&ctx);
                    out.set("done", true).ok();
                    out.set("value", JSValue::undefined(&ctx)).ok();
                    Ok(out)
                }
            } else {
                Err(RongJSError::TypeError("Not ReadableStream".to_string()))
            }
        },
    )?;
    obj.set("next", next_fn)?;

    // [Symbol.asyncIterator] = () => this (as host function; inherits Function.prototype)
    let symbol = ctx
        .global()
        .get::<_, JSObject>("Symbol")?
        .get::<_, JSSymbol>("asyncIterator")?;
    let return_this = JSFunc::new(ctx, move |this: This<JSObject>| (*this).clone())?;
    obj.set(symbol, return_this)?;
    Ok(())
}

// Wrapper helper for clearer semantics
#[derive(Clone)]
pub struct JSReadableStream(pub JSObject);

impl JSReadableStream {
    pub fn new(ctx: &JSContext, stream: ReadableStream) -> JSResult<Self> {
        let obj = rong::Class::get::<ReadableStream>(ctx)?.instance(stream);
        install_instance_async_iter(ctx, &obj)?;
        Ok(Self(obj))
    }

    pub fn from_receiver(
        ctx: &JSContext,
        rx: mpsc::Receiver<Result<Bytes, String>>,
    ) -> JSResult<Self> {
        let stream = ReadableStream::from_receiver(rx);
        Self::new(ctx, stream)
    }

    /// Construct a ReadableStream from a shared receiver slot.
    /// The slot is an Arc<Mutex<Option<Receiver>>> managed by another owner.
    /// This does not consume the channel until the stream is locked via getReader/iteration.
    pub fn from_shared_receiver(
        ctx: &JSContext,
        slot: Arc<StdMutex<Option<mpsc::Receiver<Result<Bytes, String>>>>>,
    ) -> JSResult<Self> {
        let stream = ReadableStream { rx_slot: slot };
        Self::new(ctx, stream)
    }

    pub fn from_async_reader<R>(ctx: &JSContext, reader: R, chunk_size: usize) -> JSResult<Self>
    where
        R: AsyncRead + Unpin + Send + 'static,
    {
        let stream = ReadableStream::from_async_reader(reader, chunk_size);
        Self::new(ctx, stream)
    }

    pub fn into_object(self) -> JSObject {
        self.0
    }

    pub fn object(&self) -> JSObject {
        self.0.clone()
    }
}

pub fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<ReadableStream>()?;
    ctx.register_class::<ReadableStreamDefaultReader>()?;
    ctx.register_class::<ReadableStreamDefaultController>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong_test::*;

    #[test]
    fn test_stream_js() {
        async_run!(|ctx: JSContext| async move {
            rong_assert::init(&ctx)?;
            rong_console::init(&ctx)?;
            rong_encoding::init(&ctx)?;
            rong_timer::init(&ctx)?;
            crate::init(&ctx)?;

            let passed = UnitJSRunner::load_script(&ctx, "stream.js")
                .await?
                .run()
                .await?;
            assert!(passed);
            Ok(())
        });
    }
}
