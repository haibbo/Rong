use bytes::{Bytes, BytesMut};
use http::{HeaderMap, Method, Uri, header};
use http_body_util::BodyExt;

use rong::{function::Optional, *};

use crate::body::{BodyKind, HttpBody};
use crate::header::Headers;
use rong_abort::AbortReceiver;
use rong_buffer::Blob;
use rong_stream::JSReadableStream;
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
    content_type: Option<String>,
    content_encoding: Option<String>,
    abort_receiver: Option<AbortReceiver>,
}

#[derive(FromJSValue)]
struct InitOption {
    status: Option<u16>,
    status_text: Option<String>,
    headers: Option<Headers>,
}

impl TryFromJSValue for InitOption {
    fn try_from_js(value: JSValue) -> JSResult<Self> {
        let obj = value.into_object().ok_or(RongJSError::TypeError(
            "Invalid Response Option".to_string(),
        ))?;

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
                    return Err(RongJSError::TypeError(format!(
                        "Invalid status code: {}",
                        status
                    )));
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
        match &self.body {
            Some(BodyKind::Hyper(inner)) => inner.lock().map(|g| g.is_none()).unwrap_or(true),
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
            content_type: self.content_type.clone(),
            content_encoding: self.content_encoding.clone(),
            abort_receiver: self.abort_receiver.clone(),
        }
    }

    #[js_method(getter)]
    fn body(&self, ctx: JSContext) -> Option<JSObject> {
        match &self.body {
            Some(BodyKind::Channel(inner)) => {
                let maybe_rx = inner.lock().ok().and_then(|mut g| g.take());
                if let Some(rx) = maybe_rx {
                    JSReadableStream::from_receiver(&ctx, rx)
                        .map(|jsrs| jsrs.into_object())
                        .ok()
                } else {
                    None
                }
            }
            Some(BodyKind::Buffered(b)) => {
                let (tx, rx) = mpsc::channel::<Result<Bytes, String>>(1);
                let bytes = b.clone();
                tokio::spawn(async move {
                    let _ = tx.send(Ok(bytes)).await;
                });
                JSReadableStream::from_receiver(&ctx, rx)
                    .map(|jsrs| jsrs.into_object())
                    .ok()
            }
            _ => None,
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
                let maybe_rx = inner
                    .lock()
                    .map(|mut g| g.take())
                    .map_err(|_| RongJSError::Error("Failed to lock channel body".to_string()))?;
                if let Some(mut rx) = maybe_rx {
                    if let Some(receiver) = &mut self.abort_receiver {
                        loop {
                            tokio::select! {
                                chunk = rx.recv() => {
                                    match chunk {
                                        Some(Ok(bytes)) => collected.extend_from_slice(&bytes),
                                        Some(Err(e)) => { return Err(RongJSError::Error(e)); }
                                        None => break,
                                    }
                                }
                                abort_reason = receiver.recv() => {
                                    return Err(RongJSError::from_jsvalue(abort_reason));
                                }
                            }
                        }
                    } else {
                        while let Some(item) = rx.recv().await {
                            match item {
                                Ok(bytes) => collected.extend_from_slice(&bytes),
                                Err(e) => {
                                    return Err(RongJSError::Error(e));
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
            Some(BodyKind::Hyper(inner)) => {
                // Take the body from the shared slot, then drop the lock before await
                let maybe_body = inner
                    .lock()
                    .map(|mut g| g.take())
                    .map_err(|_| RongJSError::Error("Failed to lock response body".to_string()))?;
                if let Some(mut body) = maybe_body {
                    // Give the runtime a chance to drive the connection before reading
                    tokio::task::yield_now().await;
                    let mut buf = BytesMut::new();
                    // Get a reference to the headers for decompression after full read
                    let header_map = self.headers.as_header_map();

                    if let Some(receiver) = &mut self.abort_receiver {
                        loop {
                            tokio::select! {
                                maybe = body.frame() => {
                                    match maybe {
                                        Some(Ok(frame)) => {
                                            if let Some(data) = frame.data_ref() {
                                                buf.extend_from_slice(data);
                                            }
                                            // ignore trailers for now
                                        }
                                        Some(Err(e)) => {
                                            return Err(RongJSError::Error(format!("Failed to read body frame: {}", e)));
                                        }
                                        None => { break; }
                                    }
                                }
                                abort_reason = receiver.recv() => {
                                    return Err(RongJSError::from_jsvalue(abort_reason));
                                }
                            }
                        }
                    } else {
                        while let Some(frame) = body.frame().await {
                            let frame = frame.map_err(|e| {
                                RongJSError::Error(format!("Failed to read body frame: {}", e))
                            })?;
                            if let Some(data) = frame.data_ref() {
                                buf.extend_from_slice(data);
                            }
                        }
                    }

                    let out = crate::body::decompress_bytes(buf.freeze(), header_map)?;
                    Ok(out)
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
            return Err(RongJSError::TypeError(
                "body used already for: text".to_string(),
            ));
        }
        self.consumed = true;
        match &mut self.body {
            Some(BodyKind::JS(body)) => body.text().await,
            Some(BodyKind::Buffered(b)) => Ok(String::from_utf8_lossy(b.as_ref()).into_owned()),
            Some(BodyKind::Hyper(_)) | Some(BodyKind::Channel(_)) => {
                let bytes = self.body_to_bytes().await?;
                Ok(String::from_utf8_lossy(&bytes).into_owned())
            }
            None => Ok(String::new()),
        }
    }

    #[js_method]
    async fn json(&mut self, ctx: JSContext) -> JSResult<JSValue> {
        if self.body_used() {
            return Err(RongJSError::TypeError(
                "body used already for: json".to_string(),
            ));
        }
        self.consumed = true;
        let bytes = self.body_to_bytes().await?;
        let text = String::from_utf8_lossy(&bytes).into_owned();
        text.as_str().json_to_jsvalue(&ctx)
    }

    #[js_method]
    async fn blob(&mut self) -> JSResult<Blob> {
        if self.body_used() {
            return Err(RongJSError::TypeError(
                "body used already for: blob".to_string(),
            ));
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
            return Err(RongJSError::TypeError(
                "body used already for: arrayBuffer".to_string(),
            ));
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
            return Err(RongJSError::TypeError(format!(
                "Invalid redirect status: {}",
                status
            )));
        }

        let uri = Uri::try_from(url.as_str())
            .map_err(|_| RongJSError::TypeError(format!("Invalid URL: {}", url)))?;

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
}

impl Response {
    // Parse Content-Type header to extract mime type, charset, etc. (kept for future use)
    fn parse_content_type(headers: &HeaderMap) -> (Option<String>, Option<String>) {
        if let Some(content_type) = headers.get(header::CONTENT_TYPE) {
            if let Ok(content_type) = content_type.to_str() {
                let parts: Vec<&str> = content_type.split(';').map(|s| s.trim()).collect();
                let mime_type = parts[0].to_string();
                let charset = parts
                    .iter()
                    .find(|p| p.starts_with("charset="))
                    .map(|p| p[8..].to_string());
                return (Some(mime_type), charset);
            }
        }
        (None, None)
    }

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

        let (content_type, _) = Self::parse_content_type(&headers_in);
        let content_encoding = headers_in
            .get(header::CONTENT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        Self {
            url,
            method,
            headers,
            status: status.as_u16(),
            status_text: status.canonical_reason().unwrap_or("").to_string(),
            body: Some(body_kind),
            consumed: false,
            redirected: false,
            content_type,
            content_encoding,
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
