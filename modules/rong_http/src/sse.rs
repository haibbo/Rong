use rong::function::*;
use rong::*;
use rong_event::{Emitter, EmitterExt, EventEmitter, EventKey};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};

use crate::security::grant_network_access;

const CONNECTING: u8 = 0;
const OPEN: u8 = 1;
const CLOSED: u8 = 2;

fn type_error(message: impl Into<String>) -> RongJSError {
    HostError::new(rong::error::E_TYPE, message)
        .with_name("TypeError")
        .into()
}

#[derive(Clone)]
struct PropertyHandler {
    original: JSFunc,
    listener: JSFunc,
}

#[js_export]
pub struct EventSource {
    events: EventEmitter,
    url: String,
    ready_state: Arc<AtomicU8>,
    on_open: Arc<Mutex<Option<PropertyHandler>>>,
    on_message: Arc<Mutex<Option<PropertyHandler>>>,
    on_error: Arc<Mutex<Option<PropertyHandler>>>,
    /// Our own close sender (used by `close()` to signal abort to rong_rt).
    close_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    /// The internal close sender from rong_rt::sse::SseConnection.
    /// Must be kept alive to prevent the worker from shutting down.
    _rt_close_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    last_event_id: Arc<Mutex<String>>,
    opened_rx: Arc<Mutex<Option<oneshot::Receiver<Result<String, String>>>>>,
    rx: Arc<Mutex<Option<mpsc::Receiver<Result<rong_rt::sse::SseEvent, String>>>>>,
}

impl Emitter for EventSource {
    fn get_event_emitter(&self) -> EventEmitter {
        self.events.clone()
    }
}

#[js_class]
impl EventSource {
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

            if opts.has("reconnect") {
                if let Ok(reconnect_obj) = opts.get::<_, JSObject>("reconnect") {
                    reconnect_opts.enabled =
                        reconnect_obj.get::<_, bool>("enabled").unwrap_or(true);
                    if let Ok(v) = reconnect_obj.get::<_, f64>("baseDelayMs") {
                        if v.is_finite() && v >= 0.0 {
                            reconnect_opts.base_delay =
                                std::time::Duration::from_millis((v as u64).max(1));
                        }
                    }
                    if let Ok(v) = reconnect_obj.get::<_, f64>("maxDelayMs") {
                        if v.is_finite() && v >= 1.0 {
                            reconnect_opts.max_delay = std::time::Duration::from_millis(v as u64);
                        }
                    }
                    reconnect_opts.max_retries = reconnect_obj
                        .get::<_, f64>("maxRetries")
                        .ok()
                        .filter(|v| v.is_finite() && *v >= 0.0)
                        .map(|v| v as u32);
                }
            }

            if let Ok(v) = opts.get::<_, f64>("requestTimeoutMs") {
                if v.is_finite() && v > 0.0 {
                    request_timeout = Some(std::time::Duration::from_millis(v as u64));
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

        let rt_conn = rong_rt::sse::connect_sse(rt_options, Some(close_rx)).map_err(|e| {
            HostError::new(rong::error::E_IO, format!("failed to connect SSE: {}", e))
                .with_name("TypeError")
        })?;
        let (rx, rt_close_tx, opened_rx) = rt_conn.into_parts_with_open();

        Ok(Self {
            events: EventEmitter::new(),
            url,
            ready_state: Arc::new(AtomicU8::new(CONNECTING)),
            on_open: Arc::new(Mutex::new(None)),
            on_message: Arc::new(Mutex::new(None)),
            on_error: Arc::new(Mutex::new(None)),
            close_tx: Arc::new(Mutex::new(Some(close_tx))),
            _rt_close_tx: Arc::new(Mutex::new(rt_close_tx)),
            last_event_id: Arc::new(Mutex::new(String::new())),
            opened_rx: Arc::new(Mutex::new(Some(opened_rx))),
            rx: Arc::new(Mutex::new(Some(rx))),
        })
    }

    #[js_method]
    fn close(&self) {
        self.ready_state.store(CLOSED, Ordering::Relaxed);
        // Signal abort to rong_rt worker
        if let Ok(mut guard) = self.close_tx.lock() {
            if let Some(tx) = guard.take() {
                let _ = tx.send(());
            }
        }
        // Drop the internal close sender to stop the worker
        if let Ok(mut guard) = self._rt_close_tx.lock() {
            guard.take();
        }
    }

    #[js_method(getter, rename = "readyState")]
    fn ready_state(&self) -> u8 {
        self.ready_state.load(Ordering::Relaxed)
    }

    #[js_method(getter)]
    fn url(&self) -> String {
        self.url.clone()
    }

    #[js_method(getter, rename = "lastEventId")]
    fn last_event_id(&self) -> String {
        self.last_event_id
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    #[js_method(getter, enumerable, rename = "onopen")]
    fn get_on_open(&self, ctx: JSContext) -> JSValue {
        Self::get_handler_value(&self.on_open, &ctx)
    }

    #[js_method(setter, rename = "onopen")]
    fn set_on_open(&self, this: This<JSObject>, value: JSValue) -> JSResult<()> {
        Self::set_event_handler(this, "open", &self.on_open, value)
    }

    #[js_method(getter, enumerable, rename = "onmessage")]
    fn get_on_message(&self, ctx: JSContext) -> JSValue {
        Self::get_handler_value(&self.on_message, &ctx)
    }

    #[js_method(setter, rename = "onmessage")]
    fn set_on_message(&self, this: This<JSObject>, value: JSValue) -> JSResult<()> {
        Self::set_event_handler(this, "message", &self.on_message, value)
    }

    #[js_method(getter, enumerable, rename = "onerror")]
    fn get_on_error(&self, ctx: JSContext) -> JSValue {
        Self::get_handler_value(&self.on_error, &ctx)
    }

    #[js_method(setter, rename = "onerror")]
    fn set_on_error(&self, this: This<JSObject>, value: JSValue) -> JSResult<()> {
        Self::set_event_handler(this, "error", &self.on_error, value)
    }

    /// Internal method called by JS wrapper to start the event pump.
    /// Needs `this` (the JSObject) to dispatch events on.
    #[js_method(rename = "_start")]
    fn start(&self, this: This<JSObject>, ctx: JSContext) {
        let mut rx = match self.rx.lock() {
            Ok(mut guard) => match guard.take() {
                Some(rx) => rx,
                None => return,
            },
            Err(_) => return,
        };

        let ctx = ctx.clone();
        let es_obj = this.0.clone();
        let rs = self.ready_state.clone();
        let last_event_id = self.last_event_id.clone();
        let opened_rx = match self.opened_rx.lock() {
            Ok(mut guard) => match guard.take() {
                Some(rx) => rx,
                None => return,
            },
            Err(_) => return,
        };

        spawn(async move {
            match opened_rx.await {
                Ok(Ok(origin)) => {
                    if rs.load(Ordering::Relaxed) == CLOSED {
                        return;
                    }
                    rs.store(OPEN, Ordering::Relaxed);

                    let open_obj = JSObject::new(&ctx);
                    let _ = open_obj.set("type", "open");
                    let _ = open_obj.set("origin", origin);
                    let _ = EventSource::do_emit(
                        This(es_obj.clone()),
                        EventKey::from("open"),
                        Rest(vec![JSValue::from(&ctx, open_obj)]),
                    );
                }
                Ok(Err(message)) => {
                    if rs.load(Ordering::Relaxed) != CLOSED {
                        let err_obj = JSObject::new(&ctx);
                        let _ = err_obj.set("type", "error");
                        let _ = err_obj.set("message", message.as_str());
                        let err_val = JSValue::from(&ctx, err_obj);

                        let _ = EventSource::do_emit(
                            This(es_obj.clone()),
                            EventKey::from("error"),
                            Rest(vec![err_val]),
                        );
                        rs.store(CLOSED, Ordering::Relaxed);
                    }
                    return;
                }
                Err(_) => {
                    if rs.load(Ordering::Relaxed) != CLOSED {
                        rs.store(CLOSED, Ordering::Relaxed);
                    }
                    return;
                }
            }

            while let Some(result) = rx.recv().await {
                if rs.load(Ordering::Relaxed) == CLOSED {
                    break;
                }

                match result {
                    Ok(evt) => {
                        if let Some(ref id) = evt.id {
                            if let Ok(mut last_id) = last_event_id.lock() {
                                *last_id = id.clone();
                            }
                        }

                        let event_obj = JSObject::new(&ctx);
                        let _ = event_obj.set("type", evt.event.as_str());
                        let _ = event_obj.set("data", evt.data.as_str());
                        let _ = event_obj.set("lastEventId", evt.id.as_deref().unwrap_or(""));
                        let _ = event_obj.set("origin", evt.origin.as_str());

                        let event_val = JSValue::from(&ctx, event_obj);

                        let _ = EventSource::do_emit(
                            This(es_obj.clone()),
                            EventKey::from(evt.event.as_str()),
                            Rest(vec![event_val]),
                        );
                    }
                    Err(e) => {
                        let err_obj = JSObject::new(&ctx);
                        let _ = err_obj.set("type", "error");
                        let _ = err_obj.set("message", e.as_str());
                        let err_val = JSValue::from(&ctx, err_obj);

                        let _ = EventSource::do_emit(
                            This(es_obj.clone()),
                            EventKey::from("error"),
                            Rest(vec![err_val]),
                        );
                        break;
                    }
                }
            }

            rs.store(CLOSED, Ordering::Relaxed);
        });
    }

    #[js_method(gc_mark)]
    fn gc_mark_with<F>(&self, mark_fn: F)
    where
        F: FnMut(&JSValue),
    {
        let mut mark_fn = mark_fn;
        self.events.gc_mark_with(|value| mark_fn(value));

        for slot in [&self.on_open, &self.on_message, &self.on_error] {
            if let Some(handler) = slot.lock().unwrap_or_else(|e| e.into_inner()).clone() {
                for func in [handler.original, handler.listener] {
                    let value = func.clone().into_js_value(&func.get_ctx());
                    mark_fn(&value);
                }
            }
        }
    }
}

impl EventSource {
    fn get_handler_value(slot: &Arc<Mutex<Option<PropertyHandler>>>, ctx: &JSContext) -> JSValue {
        slot.lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .map(|handler| handler.original.into_js_value(ctx))
            .unwrap_or_else(|| JSValue::null(ctx))
    }

    fn set_event_handler(
        this: This<JSObject>,
        event_name: &str,
        slot: &Arc<Mutex<Option<PropertyHandler>>>,
        value: JSValue,
    ) -> JSResult<()> {
        let key = EventKey::from(event_name);
        let next = if value.is_null() || value.is_undefined() {
            None
        } else {
            Some(Self::create_property_handler(&this.0, value.try_into()?)?)
        };

        let prev = {
            let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
            let prev = guard.take();
            *guard = next.clone();
            prev
        };

        if let Some(prev) = prev {
            Self::remove_event_listener(This(this.0.clone()), key.clone(), prev.listener)?;
        }

        if let Some(next) = next {
            Self::add_event_listener(this, key, next.listener.clone(), false, false)?;
        }

        Ok(())
    }

    fn create_property_handler(target: &JSObject, original: JSFunc) -> JSResult<PropertyHandler> {
        let ctx = target.get_ctx();
        let original_for_listener = original.clone();
        let listener = JSFunc::new(
            &ctx,
            move |this: This<JSObject>, args: Rest<JSValue>| -> JSResult<()> {
                original_for_listener.call::<_, ()>(Some(this.0.clone()), (args.0.clone(),))
            },
        )?;

        Ok(PropertyHandler { original, listener })
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
    ctx.register_class::<EventSource>()?;
    EventSource::add_web_event_target_prototype(ctx)?;

    let ctor = Class::get::<EventSource>(ctx)?;
    ctor.set("CONNECTING", CONNECTING as u32)?;
    ctor.set("OPEN", OPEN as u32)?;
    ctor.set("CLOSED", CLOSED as u32)?;

    let proto: JSObject = ctor.get("prototype")?;
    proto.set("CONNECTING", CONNECTING as u32)?;
    proto.set("OPEN", OPEN as u32)?;
    proto.set("CLOSED", CLOSED as u32)?;

    // Wrap the constructor so that _start() is called automatically after construction.
    // This is needed because _start() requires `this` (the JSObject), which is not
    // available inside the Rust constructor.
    ctx.eval::<()>(Source::from_bytes(
        r#"(function() {
            const _ES = EventSource;
            const _proto = _ES.prototype;
            globalThis.EventSource = function EventSource(url, opts) {
                const es = opts !== undefined ? new _ES(url, opts) : new _ES(url);
                es._start();
                return es;
            };
            globalThis.EventSource.prototype = _proto;
            globalThis.EventSource.CONNECTING = 0;
            globalThis.EventSource.OPEN = 1;
            globalThis.EventSource.CLOSED = 2;
        })();"#,
    ))?;

    Ok(())
}
