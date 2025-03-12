use bytes::Bytes;
use http::{header, HeaderMap, Method, Uri};
use http_body_util::BodyExt;
use hyper::body::Incoming;
use rusty_js::{function::Optional, *};

use crate::body::{BodyKind, HttpBody};
use crate::header::Headers;
use abort::AbortReceiver;
use buffer::Blob;

#[derive(Default)]
#[js_export]
pub struct Response {
    url: Uri,
    method: Method,
    headers: Headers,
    status: u16,
    status_text: String,
    body: Option<BodyKind>,
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
        let obj = value.into_object().ok_or(RustyJSError::TypeError(
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
            ..Default::default()
        };

        if let Some(body) = body.0 {
            response.body = Some(BodyKind::JS(HttpBody(body)));
        }

        if let Some(init) = init.0 {
            if let Some(status) = init.status {
                // Validate status code
                if !(100..=599).contains(&status) {
                    return Err(RustyJSError::TypeError(format!(
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
        match &self.body {
            Some(BodyKind::Hyper(body)) => body.is_none(),
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
            redirected: self.redirected,
            content_type: self.content_type.clone(),
            content_encoding: self.content_encoding.clone(),
            abort_receiver: self.abort_receiver.clone(),
        }
    }

    async fn body_to_bytes(&mut self) -> JSResult<Bytes> {
        match &mut self.body {
            Some(BodyKind::JS(body)) => body.bytes().await,
            Some(BodyKind::Hyper(body)) => {
                if let Some(body) = body.take() {
                    // Check for abort signal before starting to read
                    if let Some(receiver) = &mut self.abort_receiver {
                        tokio::select! {
                            result = body.collect() => {
                                let collected = result.map_err(|e| {
                                    RustyJSError::Error(format!("Failed to collect body: {}", e))
                                })?;
                                let bytes = collected.to_bytes();
                                // Get a reference to the headers for decompression
                                let header_map = self.headers.as_header_map();
                                crate::body::decompress_bytes(bytes, header_map)
                            }
                            abort_reason = receiver.recv() => {
                                Err(RustyJSError::from_jsvalue(abort_reason))
                            }
                        }
                    } else {
                        let collected = body.collect().await.map_err(|e| {
                            RustyJSError::Error(format!("Failed to collect body: {}", e))
                        })?;
                        let bytes = collected.to_bytes();
                        // Get a reference to the headers for decompression
                        let header_map = self.headers.as_header_map();
                        crate::body::decompress_bytes(bytes, header_map)
                    }
                } else {
                    Ok(Bytes::new())
                }
            }
            None => Ok(Bytes::new()),
        }
    }

    #[js_method]
    async fn text(&mut self) -> JSResult<String> {
        match &mut self.body {
            Some(BodyKind::JS(body)) => body.text().await,
            Some(BodyKind::Hyper(_)) => {
                let bytes = self.body_to_bytes().await?;
                Ok(String::from_utf8_lossy(&bytes).into_owned())
            }
            None => Ok(String::new()),
        }
    }

    #[js_method]
    async fn json(&mut self, ctx: JSContext) -> JSResult<JSValue> {
        let text = self.text().await?;
        text.as_str().json_to_jsvalue(&ctx)
    }

    #[js_method]
    async fn blob(&mut self) -> JSResult<Blob> {
        let bytes = self.body_to_bytes().await?;
        let mime = self
            .headers
            .get("Content-Type".to_string())?
            .unwrap_or_else(|| "".to_string());
        Ok(Blob::from_parts(mime, bytes.to_vec()))
    }

    #[js_method(rename = "arrayBuffer")]
    async fn array_buffer(&mut self, ctx: JSContext) -> JSResult<JSArrayBuffer<u8>> {
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
            return Err(RustyJSError::TypeError(format!(
                "Invalid redirect status: {}",
                status
            )));
        }

        let uri = Uri::try_from(url.as_str())
            .map_err(|_| RustyJSError::TypeError(format!("Invalid URL: {}", url)))?;

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
    // Parse Content-Type header to extract mime type, charset, etc.
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

    pub(crate) fn from_hyper(
        response: hyper::Response<Incoming>,
        abort_receiver: Option<AbortReceiver>,
    ) -> Self {
        let (parts, body) = response.into_parts();

        // Convert hyper headers to Headers
        let mut headers = Headers::default();
        for (name, value) in parts.headers.iter() {
            if let Ok(value_str) = value.to_str() {
                let _ = headers.set(name.to_string(), value_str.to_string());
            }
        }

        // Parse content type and charset
        let (content_type, _) = Self::parse_content_type(&parts.headers);

        // Get content encoding
        let content_encoding = parts
            .headers
            .get(header::CONTENT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        Self {
            url: Uri::default(), // URI comes from request, not response
            status: parts.status.as_u16(),
            status_text: parts.status.canonical_reason().unwrap_or("").to_string(),
            headers,
            body: Some(BodyKind::Hyper(Some(body))),
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
    use rustyjs_test::*;

    #[test]
    fn test_response() {
        async_run!(|ctx: JSContext| async move {
            assert::init(&ctx)?;
            console::init(&ctx)?;
            encoding::init(&ctx)?;
            lxr_url::init(&ctx)?;

            buffer::init(&ctx)?;
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
