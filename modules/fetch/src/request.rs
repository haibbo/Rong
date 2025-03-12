use http::{Method, Uri};
use rusty_js::{function::Optional, *};

use crate::body::HttpBody;
use crate::header::Headers;
use abort::AbortSignal;
use lxr_url::URL;

#[js_export]
pub struct Request {
    pub(crate) url: Uri,
    pub(crate) method: Method,
    pub(crate) headers: Headers,
    pub(crate) body: Option<HttpBody>,
    redirect: RequestRedirect,
    signal: Option<AbortSignal>, // AbortSignal
}

impl Request {
    pub(crate) fn abort_signal(&self) -> Option<&AbortSignal> {
        self.signal.as_ref()
    }
}

#[derive(Default, Clone)]
enum RequestRedirect {
    #[default]
    Follow,
    Error,
    Manual,
}

#[derive(FromJSValue, Default)]
pub(crate) struct RequestInit {
    method: Option<Method>,
    headers: Option<Headers>,
    body: Option<JSValue>,
    redirect: Option<RequestRedirect>,
    signal: Option<AbortSignal>, // AbortSignal
}

impl RequestInit {
    fn assign_request(self, request: &mut Request) {
        if let Some(method) = self.method {
            request.method = method;
        }
        if let Some(headers) = self.headers {
            request.headers = headers;
        }
        if let Some(body) = self.body {
            request.body = Some(HttpBody(body));
        }
        if let Some(redirect) = self.redirect {
            request.redirect = redirect;
        }
        if let Some(signal) = self.signal {
            request.signal = Some(signal);
        }
    }
}

impl TryFromJSValue for RequestInit {
    fn try_from_js(value: JSValue) -> JSResult<Self> {
        let mut request = Self::default();

        let obj = value
            .into_object()
            .ok_or(RustyJSError::TypeError("Invalid RequestInit".to_string()))?;

        // Method
        if let Ok(method_str) = obj.get::<_, String>("method") {
            request.method =
                Some(Method::from_bytes(method_str.as_bytes()).map_err(|_| {
                    RustyJSError::TypeError(format!("Invalid method: {}", method_str))
                })?);
        }

        // Headers
        if let Ok(headers_init) = obj.get::<_, JSValue>("headers") {
            request.headers = Some(Headers::new(Optional(Some(headers_init)))?);
        }

        // Body
        if let Ok(body) = obj.get::<_, JSValue>("body") {
            request.body = Some(body);
        }

        // Redirect
        if let Ok(redirect_str) = obj.get::<_, String>("redirect") {
            let redirect = match redirect_str.as_str() {
                "follow" => RequestRedirect::Follow,
                "error" => RequestRedirect::Error,
                "manual" => RequestRedirect::Manual,
                _ => {
                    return Err(RustyJSError::TypeError(format!(
                        "Invalid redirect: {}",
                        redirect_str
                    )))
                }
            };
            request.redirect = Some(redirect);
        }

        // Signal
        if let Ok(signal) = obj.get::<_, AbortSignal>("signal") {
            request.signal = Some(signal);
        }
        Ok(request)
    }
}

#[js_class]
impl Request {
    #[js_method(constructor)]
    pub(crate) fn new(input: JSValue, request_init: Optional<RequestInit>) -> JSResult<Self> {
        // Parse input - can be a URL string or another Request object
        let mut request = if let Ok(url_str) = input.clone().try_into::<String>() {
            // Validate URL format
            if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
                return Err(RustyJSError::TypeError(format!("Invalid URL: {}", url_str)));
            }
            let url = Uri::try_from(url_str.as_str())
                .map_err(|_| RustyJSError::TypeError(format!("Invalid URL: {}", url_str)))?;

            Self {
                url,
                ..Default::default()
            }
        } else if let Some(obj) = input.into_object() {
            if let Ok(req) = obj.borrow::<Request>() {
                req.clone()
            } else if let Ok(url) = obj.borrow::<URL>() {
                // Convert URL to string first, then parse as Uri
                let url_str = url.to_string();
                let uri = Uri::try_from(url_str.as_str())
                    .map_err(|_| RustyJSError::TypeError(format!("Invalid URL: {}", url_str)))?;
                Self {
                    url: uri,
                    ..Default::default()
                }
            } else {
                Self::default()
            }
        } else {
            Self::default()
        };

        // Process init object if provided
        if let Some(init) = request_init.0 {
            init.assign_request(&mut request);
        }

        // make sure body is None for Get
        if request.method == Method::GET {
            request.body = None;
        }

        Ok(request)
    }

    #[js_method(getter)]
    fn method(&self) -> String {
        self.method.as_str().to_string()
    }

    #[js_method(getter)]
    fn url(&self) -> String {
        self.url.to_string()
    }

    #[js_method(getter)]
    fn headers(&self) -> Headers {
        self.headers.clone()
    }

    #[js_method(getter)]
    fn cache(&self) -> &'static str {
        "no-cache"
    }

    #[js_method(getter)]
    pub fn redirect(&self) -> &'static str {
        match self.redirect {
            RequestRedirect::Follow => "follow",
            RequestRedirect::Error => "error",
            RequestRedirect::Manual => "manual",
        }
    }

    #[js_method(getter)]
    fn keepalive(&self) -> bool {
        true
    }

    #[js_method(getter)]
    fn signal(&self) -> Option<AbortSignal> {
        self.signal.clone()
    }

    #[js_method(getter)]
    fn body(&self) -> Option<JSValue> {
        self.body.clone().map(|b| b.0)
    }

    #[js_method(getter, rename = "bodyUsed")]
    fn body_used(&self) -> bool {
        self.body.is_none()
    }

    #[js_method]
    fn clone(&self) -> Self {
        Self {
            method: self.method.clone(),
            url: self.url.clone(),
            headers: self.headers.clone(),
            body: self.body.clone(),
            redirect: self.redirect.clone(),
            signal: self.signal.clone(),
        }
    }

    #[js_method]
    async fn text(&self) -> JSResult<String> {
        if let Some(body) = &self.body {
            body.text().await
        } else {
            Ok(String::new())
        }
    }

    #[js_method]
    async fn json(&self, ctx: JSContext) -> JSResult<JSValue> {
        let text = self.text().await?;
        // Use the to_js_value() trait method to convert the string to JSValue
        text.as_str().json_to_jsvalue(&ctx)
    }

    #[js_method(rename = "arrayBuffer")]
    async fn array_buffer(&self, ctx: JSContext) -> JSResult<JSArrayBuffer<u8>> {
        if let Some(body) = &self.body {
            let bytes = body.bytes().await?;
            JSArrayBuffer::from_bytes(&ctx, &bytes)
        } else {
            JSArrayBuffer::from_bytes(&ctx, &[])
        }
    }

    #[js_method(rename = "formData")]
    async fn form_data(&self) -> JSResult<JSObject> {
        // TODO: Implement form data parsing
        Err(RustyJSError::TypeError("Not implemented".to_string()))
    }
}

impl Default for Request {
    fn default() -> Self {
        Self {
            method: Method::GET,
            url: Uri::from_static("about:blank"),
            headers: Headers::default(),
            body: None,
            redirect: RequestRedirect::default(),
            signal: None,
        }
    }
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<Request>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyjs_test::*;

    #[test]
    fn test_request() {
        async_run!(|ctx: JSContext| async move {
            assert::init(&ctx)?;
            console::init(&ctx, None)?;
            encoding::init(&ctx).unwrap();
            lxr_url::init(&ctx).unwrap();

            // Initialize Blob first
            crate::blob::init(&ctx).unwrap();
            // Then initialize Headers
            crate::header::init(&ctx).unwrap();
            // Finally initialize Request
            init(&ctx).unwrap();

            let passed = UnitJSRunner::load_script(&ctx, "request.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }
}
