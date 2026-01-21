use bytes::Bytes;
use http::{Method, Uri, header};

use rong::{function::Optional, *};

use crate::body::{BodyKind, HttpBody};
use crate::header::Headers;
use rong_abort::AbortReceiver;
use rong_buffer::Blob;
use rong_stream::{JSReadableStream, readable_stream_is_locked};
use std::cell::RefCell;
use tokio::sync::mpsc;

#[derive(Default)]
#[js_export]
pub struct Response {
    url: Uri,
    method: Method,
    headers: Headers,
    status: u16,
    status_text: String,
    body: Option<BodyKind>,
    consumed: bool,
    redirected: bool,
    abort_receiver: Option<AbortReceiver>,
    // Cache a JS ReadableStream instance so repeated Response.body access
    // returns the same object and doesn't have side effects.
    body_stream: RefCell<Option<JSObject>>,
}

#[derive(FromJSValue)]
struct InitOption {
    status: Option<u16>,
    status_text: Option<String>,
    headers: Option<Headers>,
}

impl TryFromJSValue for InitOption {
    fn try_from_js(value: JSValue) -> JSResult<Self> {
        let obj = value.into_object().ok_or_else(|| {
            HostError::new(rong::error::E_INVALID_ARG, "Invalid Response Option")
                .with_name("TypeError")
        })?;

        let status = obj.get::<_, u16>("status").ok();
        let status_text = obj.get::<_, String>("statusText").ok();
        let headers = obj
            .get::<_, JSValue>("headers")
            .map(|v| Headers::new(Optional(Some(v))).ok());

        Ok(Self {
            status,
            status_text,
            headers: headers.unwrap_or(None),
        })
    }
}

#[js_class]
impl Response {
    #[js_method(constructor)]
    fn new(body: Optional<JSValue>, init: Optional<InitOption>) -> JSResult<Self> {
        let mut response = Self {
            status: 200,
            status_text: "".to_string(),
            consumed: false,
            ..Default::default()
        };

        if let Some(body) = body.0 {
            response.body = Some(BodyKind::JS(HttpBody(body)));
        }

        if let Some(init) = init.0 {
            if let Some(status) = init.status {
                // Validate status code
                if !(100..=599).contains(&status) {
                    return Err(HostError::new(
                        rong::error::E_OUT_OF_RANGE,
                        format!("Invalid status code: {}", status),
                    )
                    .with_name("RangeError")
                    .into());
                }
                response.status = status;
            }

            if let Some(text) = init.status_text {
                response.status_text = text;
            }

            if let Some(headers) = init.headers {
                response.headers = headers;
            }
        }

        Ok(response)
    }

    #[js_method(getter)]
    fn ok(&self) -> bool {
        (200..=299).contains(&self.status)
    }

    #[js_method(getter)]
    fn status(&self) -> u16 {
        self.status
    }

    #[js_method(getter, rename = "statusText")]
    fn status_text(&self) -> String {
        self.status_text.clone()
    }

    #[js_method(getter)]
    fn headers(&self) -> Headers {
        self.headers.clone()
    }

    #[js_method(getter)]
    fn redirected(&self) -> bool {
        self.redirected
    }

    #[js_method(getter)]
    fn url(&self) -> String {
        self.url.to_string()
    }

    #[js_method(getter, rename = "bodyUsed")]
    pub fn body_used(&self) -> bool {
        if self.consumed {
            return true;
        }
        // If we have materialized a ReadableStream, check whether it is locked
        if let Some(obj) = self.body_stream.borrow().as_ref()
            && let Ok(rs) = obj.borrow::<rong_stream::ReadableStream>()
        {
            return readable_stream_is_locked(&rs);
        }
        // Fallback: before materialization, channel not taken means not used
        match &self.body {
            Some(BodyKind::Channel(inner)) => inner.lock().map(|g| g.is_none()).unwrap_or(true),
            _ => false,
        }
    }

    #[js_method(getter)]
    fn type_(&self) -> &'static str {
        "todo"
    }

    #[js_method]
    fn clone(&self) -> Self {
        Self {
            url: self.url.clone(),
            method: self.method.clone(),
            headers: self.headers.clone(),
            status: self.status,
            status_text: self.status_text.clone(),
            body: self.body.clone(),
            consumed: self.consumed,
            redirected: self.redirected,
            abort_receiver: self.abort_receiver.clone(),
            body_stream: RefCell::new(self.body_stream.borrow().clone()),
        }
    }

    #[js_method(getter)]
    fn body(&self, ctx: JSContext) -> Option<JSObject> {
        // Return cached stream if we already created one
        if let Some(obj) = self.body_stream.borrow().as_ref() {
            return Some(obj.clone());
        }

        // Create and cache a stream based on the current body kind
        match &self.body {
            Some(BodyKind::Channel(inner)) => {
                // Do not consume the receiver on property access; build a stream from the shared slot
                if let Ok(jsrs) = JSReadableStream::from_shared_receiver(&ctx, inner.clone()) {
                    let obj = jsrs.into_object();
                    self.body_stream.replace(Some(obj.clone()));
                    Some(obj)
                } else {
                    None
                }
            }
            Some(BodyKind::Buffered(b)) => {
                let (tx, rx) = mpsc::channel::<Result<Bytes, String>>(1);
                let bytes = b.clone();
                rong::spawn(async move {
                    let _ = tx.send(Ok(bytes)).await;
                });
                if let Ok(jsrs) = JSReadableStream::from_receiver(&ctx, rx) {
                    let obj = jsrs.into_object();
                    self.body_stream.replace(Some(obj.clone()));
                    Some(obj)
                } else {
                    None
                }
            }
            Some(BodyKind::JS(body)) => {
                // Materialize JS body into a one-shot stream
                let (tx, rx) = mpsc::channel::<Result<Bytes, String>>(1);
                let body_clone = body.clone();
                rong::spawn(async move {
                    match body_clone.bytes().await {
                        Ok(bytes) => {
                            let _ = tx.send(Ok(bytes)).await;
                        }
                        Err(e) => {
                            let _ = tx.send(Err(format!("{}", e))).await;
                        }
                    }
                });
                if let Ok(jsrs) = JSReadableStream::from_receiver(&ctx, rx) {
                    let obj = jsrs.into_object();
                    self.body_stream.replace(Some(obj.clone()));
                    Some(obj)
                } else {
                    None
                }
            }
            None => None,
        }
    }

    async fn body_to_bytes(&mut self) -> JSResult<Bytes> {
        match &mut self.body {
            Some(BodyKind::JS(body)) => body.bytes().await,
            Some(BodyKind::Buffered(b)) => {
                let header_map = self.headers.as_header_map();
                crate::body::decompress_bytes(b.clone(), header_map)
            }
            Some(BodyKind::Channel(inner)) => {
                let mut collected = Vec::new();
                // Pre-reserve capacity using Content-Length when available
                if let Some(cl_val) = self
                    .headers
                    .as_header_map()
                    .get(header::CONTENT_LENGTH)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| s.parse::<usize>().ok())
                {
                    collected.reserve(cl_val);
                }
                // Try to take receiver from cached ReadableStream first (if one exists)
                let mut rx_opt = if let Some(obj) = self.body_stream.borrow().as_ref() {
                    if let Ok(rs) = obj.borrow::<rong_stream::ReadableStream>() {
                        rong_stream::readable_stream_take_receiver(&rs)
                    } else {
                        None
                    }
                } else {
                    None
                };

                // Fallback to internal channel if stream is not materialized
                if rx_opt.is_none() {
                    rx_opt = inner.lock().map(|mut g| g.take()).map_err(|_| {
                        HostError::new(rong::error::E_INTERNAL, "Failed to lock channel body")
                    })?;
                }

                if let Some(mut rx) = rx_opt {
                    if let Some(receiver) = &mut self.abort_receiver {
                        loop {
                            tokio::select! {
                                chunk = rx.recv() => {
                                    match chunk {
                                        Some(Ok(bytes)) => collected.extend_from_slice(&bytes),
                                        Some(Err(e)) => {
                                            return Err(HostError::new(rong::error::E_IO, e).into());
                                        }
                                        None => break,
                                    }
                                }
                                abort_reason = receiver.recv() => {
                                    return Err(RongJSError::from_thrown_value(abort_reason));
                                }
                            }
                        }
                    } else {
                        while let Some(item) = rx.recv().await {
                            match item {
                                Ok(bytes) => collected.extend_from_slice(&bytes),
                                Err(e) => {
                                    return Err(HostError::new(rong::error::E_IO, e).into());
                                }
                            }
                        }
                    }
                    let header_map = self.headers.as_header_map();
                    crate::body::decompress_bytes(Bytes::from(collected), header_map)
                } else {
                    Ok(Bytes::new())
                }
            }

            None => Ok(Bytes::new()),
        }
    }

    #[js_method]
    async fn text(&mut self) -> JSResult<String> {
        if self.body_used() {
            return Err(HostError::new(
                rong::error::E_INVALID_STATE,
                "body used already for: text",
            )
            .with_name("TypeError")
            .into());
        }
        self.consumed = true;
        let bytes = self.body_to_bytes().await?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    #[js_method]
    async fn json(&mut self, ctx: JSContext) -> JSResult<JSValue> {
        if self.body_used() {
            return Err(HostError::new(
                rong::error::E_INVALID_STATE,
                "body used already for: json",
            )
            .with_name("TypeError")
            .into());
        }
        self.consumed = true;
        let bytes = self.body_to_bytes().await?;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        text.as_str().json_to_js_value(&ctx)
    }

    #[js_method]
    async fn blob(&mut self) -> JSResult<Blob> {
        if self.body_used() {
            return Err(HostError::new(
                rong::error::E_INVALID_STATE,
                "body used already for: blob",
            )
            .with_name("TypeError")
            .into());
        }
        self.consumed = true;
        let bytes = self.body_to_bytes().await?;
        let mime = self
            .headers
            .get("Content-Type".to_string())?
            .unwrap_or_else(|| "".to_string());
        Ok(Blob::from_parts(mime, bytes.to_vec()))
    }

    #[js_method(rename = "arrayBuffer")]
    async fn array_buffer(&mut self, ctx: JSContext) -> JSResult<JSArrayBuffer<u8>> {
        if self.body_used() {
            return Err(HostError::new(
                rong::error::E_INVALID_STATE,
                "body used already for: arrayBuffer",
            )
            .with_name("TypeError")
            .into());
        }
        self.consumed = true;
        let bytes = self.body_to_bytes().await?;
        JSArrayBuffer::from_bytes(&ctx, &bytes)
    }

    #[js_method]
    fn error() -> Self {
        Self {
            status: 0,
            status_text: String::new(),
            ..Default::default()
        }
    }

    #[js_method]
    fn redirect(url: String, status: Optional<u16>) -> JSResult<Self> {
        let status = status.0.unwrap_or(302);

        // Validate redirect status
        if !matches!(status, 301 | 302 | 303 | 307 | 308) {
            return Err(HostError::new(
                rong::error::E_INVALID_ARG,
                format!("Invalid redirect status: {}", status),
            )
            .with_name("TypeError")
            .into());
        }

        let uri = Uri::try_from(url.as_str()).map_err(|_| {
            HostError::new(rong::error::E_INVALID_ARG, format!("Invalid URL: {}", url))
                .with_name("TypeError")
        })?;

        let mut headers = Headers::default();
        headers.set("Location".to_string(), url)?;

        Ok(Self {
            url: uri,
            status,
            headers,
            redirected: true,
            ..Default::default()
        })
    }

    #[js_method(gc_mark)]
    fn gc_mark_with<F>(&self, mark_fn: F)
    where
        F: FnMut(&JSValue),
    {
        // Mark any JS values reachable from Response so the GC keeps them alive
        // - BodyKind::JS holds an HttpBody which wraps a JSValue
        // - abort_receiver may hold a JSValue reason inside the watch channel
        // - Cached body_stream JSObject if created
        let mut mark_fn = mark_fn;
        if let Some(BodyKind::JS(js_body)) = &self.body {
            mark_fn(&js_body.0);
        }

        if let Some(receiver) = &self.abort_receiver {
            receiver.gc_mark_with(|v| mark_fn(v));
        }

        if let Some(obj) = self.body_stream.borrow().as_ref() {
            mark_fn(obj);
        }
    }
}

impl Response {
    pub(crate) fn from_meta(
        status: http::StatusCode,
        headers_in: http::HeaderMap,
        body_kind: BodyKind,
        abort_receiver: Option<AbortReceiver>,
        method: Method,
        url: Uri,
    ) -> Self {
        // Convert hyper headers to Headers
        let mut headers = Headers::default();
        for (name, value) in headers_in.iter() {
            if let Ok(value_str) = value.to_str() {
                let _ = headers.set(name.to_string(), value_str.to_string());
            }
        }

        Self {
            url,
            method,
            headers,
            status: status.as_u16(),
            status_text: status.canonical_reason().unwrap_or("").to_string(),
            body: Some(body_kind),
            consumed: false,
            redirected: false,
            abort_receiver,
            ..Default::default()
        }
    }
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<Response>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong_test::*;

    #[test]
    fn test_response() {
        async_run!(|ctx: JSContext| async move {
            rong_assert::init(&ctx)?;
            rong_console::init(&ctx)?;
            rong_encoding::init(&ctx)?;
            rong_url::init(&ctx)?;

            rong_buffer::init(&ctx)?;
            crate::header::init(&ctx)?;
            init(&ctx)?;

            let passed = UnitJSRunner::load_script(&ctx, "response.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }
}
