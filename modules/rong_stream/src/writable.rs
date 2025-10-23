use bytes::Bytes;
use rong::*;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::io::AsyncWrite;
use tokio::sync::{Mutex, mpsc, oneshot};

#[js_export]
pub struct WritableStream {
    // A single writer lock; getWriter() takes ownership.
    pub(crate) tx_slot: Arc<StdMutex<Option<mpsc::Sender<Bytes>>>>,
    // Optional completion signal for async writer (Some when created via to_async_writer)
    pub(crate) done_slot: Arc<StdMutex<Option<oneshot::Receiver<Result<(), String>>>>>,
    // Error slot to surface background write failures to the front-end
    pub(crate) err_slot: Arc<StdMutex<Option<String>>>,
    // For JS-underlying sink mode
    sink_slot: Arc<StdMutex<Option<JSObject>>>,
}

#[js_export]
pub struct WritableStreamDefaultWriter {
    // Reference back to the stream's slot to support releaseLock
    slot: Arc<StdMutex<Option<mpsc::Sender<Bytes>>>>,
    // Sender owned while the writer is locked
    tx: Arc<Mutex<Option<mpsc::Sender<Bytes>>>>,
    // Optional completion signal for close()
    done_rx: Arc<StdMutex<Option<oneshot::Receiver<Result<(), String>>>>>,
    // Reference to stream's done slot to return it on releaseLock
    done_slot_ref: Arc<StdMutex<Option<oneshot::Receiver<Result<(), String>>>>>,
    // Shared error slot to report background errors
    err_slot: Arc<StdMutex<Option<String>>>,
    // For JS sink mode: reference back to stream's sink slot and the held sink object
    sink_slot_ref: Arc<StdMutex<Option<JSObject>>>,
    sink_obj: Option<JSObject>,
}

impl WritableStream {
    pub fn to_sender(tx: mpsc::Sender<Bytes>) -> Self {
        Self {
            tx_slot: Arc::new(StdMutex::new(Some(tx))),
            done_slot: Arc::new(StdMutex::new(None)),
            err_slot: Arc::new(StdMutex::new(None)),
            sink_slot: Arc::new(StdMutex::new(None)),
        }
    }

    pub fn to_async_writer<W>(mut writer: W) -> Self
    where
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<Bytes>(16);
        let (done_tx, done_rx) = oneshot::channel::<Result<(), String>>();
        let err_slot: Arc<StdMutex<Option<String>>> = Arc::new(StdMutex::new(None));
        let err_slot_for_task = err_slot.clone();

        tokio::task::spawn(async move {
            let mut error: Option<String> = None;
            while let Some(chunk) = rx.recv().await {
                if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut writer, &chunk).await {
                    error = Some(e.to_string());
                    break;
                }
            }
            if let Err(e) = tokio::io::AsyncWriteExt::flush(&mut writer).await {
                if error.is_none() {
                    error = Some(e.to_string());
                }
            }
            if let Some(e) = error.as_ref() {
                if let Ok(mut g) = err_slot_for_task.lock() {
                    *g = Some(e.clone());
                }
            }
            let _ = done_tx.send(match error {
                Some(e) => Err(e),
                None => Ok(()),
            });
        });

        Self {
            tx_slot: Arc::new(StdMutex::new(Some(tx))),
            done_slot: Arc::new(StdMutex::new(Some(done_rx))),
            err_slot,
            sink_slot: Arc::new(StdMutex::new(None)),
        }
    }
}

#[js_class]
impl WritableStream {
    #[js_method(constructor)]
    fn new(underlying: function::Optional<JSValue>) -> JSResult<Self> {
        // If underlying sink provided, store it; else make a dummy sink without channels
        let sink = match underlying.0 {
            Some(v) => v.into_object(),
            None => None,
        };
        Ok(Self {
            tx_slot: Arc::new(StdMutex::new(None)),
            done_slot: Arc::new(StdMutex::new(None)),
            err_slot: Arc::new(StdMutex::new(None)),
            sink_slot: Arc::new(StdMutex::new(sink)),
        })
    }

    #[js_method(rename = "getWriter")]
    pub(crate) fn get_writer(&self) -> JSResult<WritableStreamDefaultWriter> {
        let mut guard = self
            .tx_slot
            .lock()
            .map_err(|_| RongJSError::Error("Stream is poisoned".to_string()))?;
        match guard.take() {
            Some(tx) => {
                // Take done_rx if any (only for async writer)
                let done_rx = {
                    let mut d = self
                        .done_slot
                        .lock()
                        .map_err(|_| RongJSError::Error("Stream is poisoned".to_string()))?;
                    d.take()
                };
                Ok(WritableStreamDefaultWriter {
                    slot: self.tx_slot.clone(),
                    tx: Arc::new(Mutex::new(Some(tx))),
                    done_rx: Arc::new(StdMutex::new(done_rx)),
                    done_slot_ref: self.done_slot.clone(),
                    err_slot: self.err_slot.clone(),
                    sink_slot_ref: self.sink_slot.clone(),
                    sink_obj: None,
                })
            }
            None => {
                // Fall back to JS sink mode
                let mut sink_guard = self
                    .sink_slot
                    .lock()
                    .map_err(|_| RongJSError::Error("Stream is poisoned".to_string()))?;
                if sink_guard.is_none() {
                    return Err(RongJSError::TypeError(
                        "WritableStream is locked".to_string(),
                    ));
                }
                let obj = sink_guard.take().unwrap();
                Ok(WritableStreamDefaultWriter {
                    slot: self.tx_slot.clone(),
                    tx: Arc::new(Mutex::new(None)),
                    done_rx: Arc::new(StdMutex::new(None)),
                    done_slot_ref: self.done_slot.clone(),
                    err_slot: self.err_slot.clone(),
                    sink_slot_ref: self.sink_slot.clone(),
                    sink_obj: Some(obj),
                })
            }
        }
    }

    #[js_method]
    fn abort(&self) -> JSResult<()> {
        let mut guard = self
            .tx_slot
            .lock()
            .map_err(|_| RongJSError::Error("Stream is poisoned".to_string()))?;
        *guard = None;
        Ok(())
    }
}

#[js_class]
impl WritableStreamDefaultWriter {
    #[js_method(constructor)]
    fn new() -> JSResult<Self> {
        Err(RongJSError::TypeError("Illegal constructor".to_string()))
    }

    #[js_method]
    pub(crate) async fn write(&mut self, chunk: JSValue) -> JSResult<()> {
        // Surface background error if any
        if let Ok(err_guard) = self.err_slot.lock() {
            if let Some(e) = err_guard.as_ref() {
                return Err(RongJSError::Error(format!("WritableStream error: {}", e)));
            }
        }
        // If we have a JS sink, call its write method in JS thread
        if let Some(sink_obj) = self.sink_obj.as_ref() {
            if let Ok(write_fn) = sink_obj.get::<_, JSFunc>("write") {
                // Await if it returns a Promise
                let _r: JSValue = write_fn.call_async(None, (chunk,)).await?;
                return Ok(());
            }
        }

        // Channel mode
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
                    "write expects Uint8Array or ArrayBuffer".to_string(),
                ));
            }
        } else {
            return Err(RongJSError::TypeError(
                "write expects a TypedArray or ArrayBuffer".to_string(),
            ));
        };
        let mut slot = self.tx.lock().await;
        match slot.as_mut() {
            Some(tx) => tx
                .send(bytes)
                .await
                .map_err(|_| RongJSError::Error("WritableStream closed".to_string())),
            None => Err(RongJSError::Error(
                "Writer not acquired or closed".to_string(),
            )),
        }
    }

    #[js_method]
    pub(crate) async fn close(&mut self) -> JSResult<()> {
        // If JS sink has close
        if let Some(sink_obj) = self.sink_obj.as_ref() {
            if let Ok(close_fn) = sink_obj.get::<_, JSFunc>("close") {
                let _r: JSValue = close_fn.call_async(None, ()).await?;
            }
        }

        // Channel mode: drop sender and await completion if possible
        {
            let mut slot = self.tx.lock().await;
            *slot = None;
        }
        let rx_opt = {
            let mut d = self
                .done_rx
                .lock()
                .map_err(|_| RongJSError::Error("Stream is poisoned".to_string()))?;
            d.take()
        };
        if let Some(rx) = rx_opt {
            match rx.await {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(RongJSError::Error(e)),
                Err(_) => Ok(()),
            }
        } else {
            Ok(())
        }
    }

    #[js_method]
    pub(crate) async fn abort(&mut self) -> JSResult<()> {
        let mut slot = self.tx.lock().await;
        *slot = None;
        Ok(())
    }

    #[js_method(rename = "releaseLock")]
    pub(crate) async fn release_lock(&mut self) -> JSResult<()> {
        // Take back sender and return it to the stream's slot
        let tx_opt = {
            let mut slot = self.tx.lock().await;
            slot.take()
        };
        if let Some(tx) = tx_opt {
            let mut guard = self
                .slot
                .lock()
                .map_err(|_| RongJSError::Error("Stream is poisoned".to_string()))?;
            if guard.is_none() {
                *guard = Some(tx);
            }
        }
        // Also return done_rx if any so next writer can await close
        let done_opt = {
            let mut d = self
                .done_rx
                .lock()
                .map_err(|_| RongJSError::Error("Stream is poisoned".to_string()))?;
            d.take()
        };
        if let Some(done) = done_opt {
            let mut g = self
                .done_slot_ref
                .lock()
                .map_err(|_| RongJSError::Error("Stream is poisoned".to_string()))?;
            if g.is_none() {
                *g = Some(done);
            }
        }
        // Return JS sink to stream slot if present
        if let Some(sink_obj) = self.sink_obj.take() {
            let mut slot = self
                .sink_slot_ref
                .lock()
                .map_err(|_| RongJSError::Error("Stream is poisoned".to_string()))?;
            if slot.is_none() {
                *slot = Some(sink_obj);
            }
        }
        Ok(())
    }
}

// Public Rust helpers for other modules
pub fn writable_stream_to_sender(tx: mpsc::Sender<Bytes>) -> WritableStream {
    WritableStream::to_sender(tx)
}

pub fn writable_stream_to_async_write<W>(writer: W) -> WritableStream
where
    W: AsyncWrite + Unpin + Send + 'static,
{
    WritableStream::to_async_writer(writer)
}

pub fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<WritableStream>()?;
    ctx.register_class::<WritableStreamDefaultWriter>()?;
    Ok(())
}
