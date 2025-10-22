use bytes::Bytes;
use rong::*;
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
        tokio::task::spawn(async move {
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

#[js_class]
impl ReadableStream {
    #[js_method(constructor)]
    fn new(underlying: function::Optional<JSValue>) -> JSResult<Self> {
        // Create a basic channel-backed stream
        let (tx, rx) = mpsc::channel::<Result<Bytes, String>>(16);
        let stream = Self::from_receiver(rx);

        // If an underlying source is provided, call start(controller) if present
        if let Some(v) = underlying.0 {
            if let Some(obj) = v.into_object() {
                // let _ctx = obj.get_ctx();
                let controller = ReadableStreamDefaultController {
                    tx: Arc::new(Mutex::new(Some(tx))),
                };
                if let Ok(start) = obj.get::<_, JSFunc>("start") {
                    let _ = start.call::<_, JSValue>(Some(obj.clone()), (controller,));
                }
            }
        }
        Ok(stream)
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

// Public Rust helpers for other modules
pub fn readable_stream_from_receiver(rx: mpsc::Receiver<Result<Bytes, String>>) -> ReadableStream {
    ReadableStream::from_receiver(rx)
}

pub fn readable_stream_from_async_read<R>(reader: R, chunk_size: usize) -> ReadableStream
where
    R: AsyncRead + Unpin + Send + 'static,
{
    ReadableStream::from_async_reader(reader, chunk_size)
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
