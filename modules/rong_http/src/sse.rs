use futures::Stream;
use rong::function::*;
use rong::*;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::sync::{mpsc, oneshot};

use crate::security::grant_network_access;

fn type_error(message: impl Into<String>) -> RongJSError {
    HostError::new(rong::error::E_TYPE, message)
        .with_name("TypeError")
        .into()
}

type EventReceiver = mpsc::Receiver<Result<rong_rt::sse::SseEvent, String>>;
type OpenedReceiver = oneshot::Receiver<Result<String, String>>;

#[allow(clippy::upper_case_acronyms, clippy::type_complexity)]
#[js_export]
pub struct SSE {
    url: String,
    close_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    _rt_close_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    rx: Arc<Mutex<Option<EventReceiver>>>,
    opened_rx: Arc<Mutex<Option<OpenedReceiver>>>,
}

#[js_class]
impl SSE {
    #[js_method(constructor)]
    fn new(_ctx: JSContext, url: String, options: Optional<JSObject>) -> JSResult<Self> {
        let parsed =
            url::Url::parse(&url).map_err(|e| type_error(format!("invalid url: {}", e)))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| type_error("url must contain a valid host"))?;
        grant_network_access(host)?;

        let destination = url_to_destination(&parsed)?;

        let mut headers = Vec::new();
        let mut reconnect_opts = rong_rt::sse::SseReconnectOptions::default();
        let mut request_timeout = None;
        let mut abort_rx: Option<oneshot::Receiver<()>> = None;

        if let Some(opts) = options.0 {
            if opts.has("headers") {
                let value = opts.get::<_, JSValue>("headers")?;
                if !value.is_undefined() && !value.is_null() {
                    let obj = value
                        .into_object()
                        .ok_or_else(|| type_error("options.headers must be an object"))?;
                    headers = obj.entries_as::<String, String>().map_err(|_| {
                        type_error("options.headers must be string key/value pairs")
                    })?;
                }
            }

            if opts.has("reconnect")
                && let Ok(reconnect_obj) = opts.get::<_, JSObject>("reconnect")
            {
                reconnect_opts.enabled = reconnect_obj.get::<_, bool>("enabled").unwrap_or(true);
                if let Ok(v) = reconnect_obj.get::<_, f64>("baseDelayMs")
                    && v.is_finite()
                    && v >= 0.0
                {
                    reconnect_opts.base_delay = std::time::Duration::from_millis((v as u64).max(1));
                }
                if let Ok(v) = reconnect_obj.get::<_, f64>("maxDelayMs")
                    && v.is_finite()
                    && v >= 1.0
                {
                    reconnect_opts.max_delay = std::time::Duration::from_millis(v as u64);
                }
                reconnect_opts.max_retries = reconnect_obj
                    .get::<_, f64>("maxRetries")
                    .ok()
                    .filter(|v| v.is_finite() && *v >= 0.0)
                    .map(|v| v as u32);
            }

            if let Ok(v) = opts.get::<_, f64>("requestTimeoutMs")
                && v.is_finite()
                && v > 0.0
            {
                request_timeout = Some(std::time::Duration::from_millis(v as u64));
            }

            // AbortSignal support
            if opts.has("signal") {
                let signal_val = opts.get::<_, JSValue>("signal")?;
                if !signal_val.is_undefined() && !signal_val.is_null() {
                    let signal_obj = signal_val
                        .into_object()
                        .ok_or_else(|| type_error("options.signal must be an AbortSignal"))?;
                    let signal = signal_obj
                        .borrow::<rong_abort::AbortSignal>()
                        .map_err(|_| type_error("options.signal must be an AbortSignal"))?;
                    let mut receiver = signal.subscribe();
                    let (tx, rx) = oneshot::channel::<()>();
                    abort_rx = Some(rx);
                    tokio::task::spawn_local(async move {
                        receiver.recv().await;
                        let _ = tx.send(());
                    });
                }
            }
        }

        let rt_options = rong_rt::sse::SseConnectOptions {
            destination,
            headers,
            last_event_id: None,
            reconnect: reconnect_opts,
            request_timeout,
        };

        let (close_tx, close_rx) = oneshot::channel::<()>();

        // Merge abort_rx and close_rx into a single signal for rong_rt
        let merged_abort_rx = if let Some(abort_rx) = abort_rx {
            let (merged_tx, merged_rx) = oneshot::channel::<()>();
            tokio::task::spawn_local(async move {
                tokio::select! {
                    _ = close_rx => {}
                    _ = abort_rx => {}
                }
                let _ = merged_tx.send(());
            });
            Some(merged_rx)
        } else {
            Some(close_rx)
        };

        let rt_conn = rong_rt::sse::connect_sse(rt_options, merged_abort_rx).map_err(|e| {
            HostError::new(rong::error::E_IO, format!("failed to connect SSE: {}", e))
                .with_name("TypeError")
        })?;
        let (rx, rt_close_tx, opened_rx) = rt_conn.into_parts_with_open();

        Ok(Self {
            url,
            close_tx: Arc::new(Mutex::new(Some(close_tx))),
            _rt_close_tx: Arc::new(Mutex::new(rt_close_tx)),
            opened_rx: Arc::new(Mutex::new(Some(opened_rx))),
            rx: Arc::new(Mutex::new(Some(rx))),
        })
    }

    #[js_method]
    fn close(&self) {
        if let Ok(mut guard) = self.close_tx.lock()
            && let Some(tx) = guard.take()
        {
            let _ = tx.send(());
        }
        if let Ok(mut guard) = self._rt_close_tx.lock() {
            guard.take();
        }
    }

    #[js_method(getter)]
    fn url(&self) -> String {
        self.url.clone()
    }

    /// Internal method called by JS wrapper to install the async iterator.
    #[js_method(rename = "_iter")]
    fn iter(&self, this: This<JSObject>, ctx: JSContext) {
        let opened_rx = self.opened_rx.lock().ok().and_then(|mut g| g.take());
        let rx = self.rx.lock().ok().and_then(|mut g| g.take());

        let stream = SseEventStream {
            ctx: ctx.clone(),
            origin: String::new(),
            state: SseStreamState::Opening,
            opened_rx,
            rx,
        };

        let _ = stream.install_js_async_iter(&ctx, &this.0);
    }

    #[js_method(gc_mark)]
    fn gc_mark_with<F>(&self, _mark_fn: F)
    where
        F: FnMut(&JSValue),
    {
    }
}

enum SseStreamState {
    Opening,
    Streaming,
    Done,
}

struct SseEventStream {
    ctx: JSContext,
    origin: String,
    state: SseStreamState,
    opened_rx: Option<OpenedReceiver>,
    rx: Option<EventReceiver>,
}

impl Stream for SseEventStream {
    type Item = JSResult<JSObject>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        loop {
            match this.state {
                SseStreamState::Opening => {
                    if let Some(ref mut opened_rx) = this.opened_rx {
                        match Pin::new(opened_rx).poll(cx) {
                            Poll::Ready(Ok(Ok(origin))) => {
                                this.origin = origin;
                                this.opened_rx = None;
                                this.state = SseStreamState::Streaming;
                                continue;
                            }
                            Poll::Ready(Ok(Err(message))) => {
                                this.state = SseStreamState::Done;
                                let err: RongJSError =
                                    HostError::new(rong::error::E_IO, message).into();
                                return Poll::Ready(Some(Err(err)));
                            }
                            Poll::Ready(Err(_)) => {
                                this.state = SseStreamState::Done;
                                let err: RongJSError =
                                    HostError::new(rong::error::E_IO, "SSE connection failed")
                                        .into();
                                return Poll::Ready(Some(Err(err)));
                            }
                            Poll::Pending => return Poll::Pending,
                        }
                    } else {
                        this.state = SseStreamState::Done;
                        return Poll::Ready(None);
                    }
                }
                SseStreamState::Streaming => {
                    if let Some(ref mut rx) = this.rx {
                        match rx.poll_recv(cx) {
                            Poll::Ready(Some(Ok(evt))) => {
                                let obj = JSObject::new(&this.ctx);
                                let _ = obj.set("type", evt.event.as_str());
                                let _ = obj.set("data", evt.data.as_str());
                                let _ = obj.set("id", evt.id.as_deref().unwrap_or(""));
                                let _ = obj.set("origin", this.origin.as_str());
                                return Poll::Ready(Some(Ok(obj)));
                            }
                            Poll::Ready(Some(Err(message))) => {
                                this.state = SseStreamState::Done;
                                let err: RongJSError =
                                    HostError::new(rong::error::E_IO, message).into();
                                return Poll::Ready(Some(Err(err)));
                            }
                            Poll::Ready(None) => {
                                this.state = SseStreamState::Done;
                                return Poll::Ready(None);
                            }
                            Poll::Pending => return Poll::Pending,
                        }
                    } else {
                        this.state = SseStreamState::Done;
                        return Poll::Ready(None);
                    }
                }
                SseStreamState::Done => return Poll::Ready(None),
            }
        }
    }
}

fn url_to_destination(parsed: &url::Url) -> JSResult<rong_rt::sse::SseDestination> {
    let scheme = match parsed.scheme() {
        "http" => rong_rt::sse::SseScheme::Http,
        "https" => rong_rt::sse::SseScheme::Https,
        _ => return Err(type_error("url scheme must be http or https")),
    };

    let target = match parsed.port() {
        Some(port) => format!("{}:{}", parsed.host_str().unwrap(), port),
        None => parsed.host_str().unwrap().to_string(),
    };

    Ok(rong_rt::sse::SseDestination {
        scheme,
        target,
        path: parsed.path().to_string(),
        query: parsed.query().map(|q| q.to_string()),
    })
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<SSE>()?;

    // Wrap the constructor so that _iter() is called automatically after construction.
    // _iter() requires `this` (the JSObject), which is not available inside the Rust constructor.
    ctx.eval::<()>(Source::from_bytes(
        r#"(function() {
            const _SSE = SSE;
            const _proto = _SSE.prototype;
            globalThis.SSE = function SSE(url, opts) {
                const sse = opts !== undefined ? new _SSE(url, opts) : new _SSE(url);
                sse._iter();
                return sse;
            };
            globalThis.SSE.prototype = _proto;
        })();"#,
    ))?;

    Ok(())
}
