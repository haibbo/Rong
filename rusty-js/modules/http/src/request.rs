use http_crate::{Method, Uri};
use rusty_js::{function::Optional, *};

use crate::{blob::Blob, header::Headers};

#[js_class]
pub struct Request {
    method: Method,
    url: Uri,
    url_string: String, // Store the original URL string
    headers: Headers,
    body: Option<Blob>,
    mode: RequestMode,
    credentials: RequestCredentials,
    cache: RequestCache,
    redirect: RequestRedirect,
    referrer: String,
    referrer_policy: ReferrerPolicy,
    integrity: String,
    keepalive: bool,
    is_reload_navigation: bool,
    is_history_navigation: bool,
    signal: Option<JSObject>, // AbortSignal
}

#[derive(Default, Clone)]
enum RequestMode {
    #[default]
    Cors,
    NoCors,
    SameOrigin,
    Navigate,
}

#[derive(Default, Clone)]
enum RequestCredentials {
    #[default]
    SameOrigin,
    Omit,
    Include,
}

#[derive(Default, Clone)]
enum RequestCache {
    #[default]
    Default,
    NoStore,
    Reload,
    NoCache,
    ForceCache,
    OnlyIfCached,
}

#[derive(Default, Clone)]
enum RequestRedirect {
    #[default]
    Follow,
    Error,
    Manual,
}

#[derive(Default, Clone)]
enum ReferrerPolicy {
    #[default]
    NoReferrer,
    NoReferrerWhenDowngrade,
    Origin,
    OriginWhenCrossOrigin,
    SameOrigin,
    StrictOrigin,
    StrictOriginWhenCrossOrigin,
    UnsafeUrl,
}

#[js_methods]
impl Request {
    fn is_valid_method(method: &str) -> bool {
        matches!(
            method.to_uppercase().as_str(),
            "GET" | "POST" | "PUT" | "DELETE" | "HEAD" | "OPTIONS" | "CONNECT" | "TRACE" | "PATCH"
        )
    }

    #[js_method(constructor)]
    pub fn new(input: JSValue, init: Optional<JSObject>) -> JSResult<Self> {
        println!("Creating new Request with input: {:?}", input);
        // Parse input - can be a URL string or another Request object
        let (method, url, url_string, headers, body) = if let Some(obj) =
            input.clone().into_object()
        {
            if let Ok(request) = obj.borrow::<Request>() {
                // Input is a Request object
                (
                    request.method.clone(),
                    request.url.clone(),
                    request.url_string.clone(),
                    request.headers.clone(),
                    request.body.clone(),
                )
            } else {
                // Input should be a URL string
                let url_str: String = input.try_into()?;
                // Validate URL format
                if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
                    return Err(RustyJSError::TypeError(format!("Invalid URL: {}", url_str)));
                }
                let url = Uri::try_from(url_str.as_str())
                    .map_err(|_| RustyJSError::TypeError(format!("Invalid URL: {}", url_str)))?;
                (Method::GET, url, url_str, Headers::default(), None)
            }
        } else {
            // Input should be a URL string
            let url_str: String = input.try_into()?;
            // Validate URL format
            if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
                return Err(RustyJSError::TypeError(format!("Invalid URL: {}", url_str)));
            }
            let url = Uri::try_from(url_str.as_str())
                .map_err(|_| RustyJSError::TypeError(format!("Invalid URL: {}", url_str)))?;
            (Method::GET, url, url_str, Headers::default(), None)
        };

        let mut request = Self {
            method,
            url,
            url_string,
            headers,
            body,
            mode: RequestMode::default(),
            credentials: RequestCredentials::default(),
            cache: RequestCache::default(),
            redirect: RequestRedirect::default(),
            referrer: String::new(),
            referrer_policy: ReferrerPolicy::default(),
            integrity: String::new(),
            keepalive: false,
            is_reload_navigation: false,
            is_history_navigation: false,
            signal: None,
        };

        // Process init object if provided
        if let Some(init) = init.0 {
            // Method
            if let Ok(method_str) = init.get::<_, String>("method") {
                if !Self::is_valid_method(&method_str) {
                    return Err(RustyJSError::TypeError(format!(
                        "Invalid method: {}",
                        method_str
                    )));
                }
                request.method = Method::from_bytes(method_str.as_bytes()).map_err(|_| {
                    RustyJSError::TypeError(format!("Invalid method: {}", method_str))
                })?;
            }

            // Headers
            if let Ok(headers_init) = init.get::<_, JSValue>("headers") {
                request.headers = Headers::new(Optional(Some(headers_init)))?;
            }

            // Body
            if let Ok(body_value) = init.get::<_, JSValue>("body") {
                if let Some(obj) = body_value.into_object() {
                    if let Ok(blob) = obj.borrow::<Blob>() {
                        request.body = Some(blob.clone());
                    }
                }
            }

            // Mode
            if let Ok(mode_str) = init.get::<_, String>("mode") {
                request.mode = match mode_str.as_str() {
                    "cors" => RequestMode::Cors,
                    "no-cors" => RequestMode::NoCors,
                    "same-origin" => RequestMode::SameOrigin,
                    "navigate" => RequestMode::Navigate,
                    _ => {
                        return Err(RustyJSError::TypeError(format!(
                            "Invalid mode: {}",
                            mode_str
                        )))
                    }
                };
            }

            // Credentials
            if let Ok(cred_str) = init.get::<_, String>("credentials") {
                request.credentials = match cred_str.as_str() {
                    "omit" => RequestCredentials::Omit,
                    "same-origin" => RequestCredentials::SameOrigin,
                    "include" => RequestCredentials::Include,
                    _ => {
                        return Err(RustyJSError::TypeError(format!(
                            "Invalid credentials: {}",
                            cred_str
                        )))
                    }
                };
            }

            // Cache
            if let Ok(cache_str) = init.get::<_, String>("cache") {
                request.cache = match cache_str.as_str() {
                    "default" => RequestCache::Default,
                    "no-store" => RequestCache::NoStore,
                    "reload" => RequestCache::Reload,
                    "no-cache" => RequestCache::NoCache,
                    "force-cache" => RequestCache::ForceCache,
                    "only-if-cached" => RequestCache::OnlyIfCached,
                    _ => {
                        return Err(RustyJSError::TypeError(format!(
                            "Invalid cache: {}",
                            cache_str
                        )))
                    }
                };
            }

            // Redirect
            if let Ok(redirect_str) = init.get::<_, String>("redirect") {
                request.redirect = match redirect_str.as_str() {
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
            }

            // Referrer
            if let Ok(referrer) = init.get::<_, String>("referrer") {
                request.referrer = referrer;
            }

            // ReferrerPolicy
            if let Ok(policy) = init.get::<_, String>("referrerPolicy") {
                request.referrer_policy = match policy.as_str() {
                    "no-referrer" => ReferrerPolicy::NoReferrer,
                    "no-referrer-when-downgrade" => ReferrerPolicy::NoReferrerWhenDowngrade,
                    "origin" => ReferrerPolicy::Origin,
                    "origin-when-cross-origin" => ReferrerPolicy::OriginWhenCrossOrigin,
                    "same-origin" => ReferrerPolicy::SameOrigin,
                    "strict-origin" => ReferrerPolicy::StrictOrigin,
                    "strict-origin-when-cross-origin" => {
                        ReferrerPolicy::StrictOriginWhenCrossOrigin
                    }
                    "unsafe-url" => ReferrerPolicy::UnsafeUrl,
                    _ => {
                        return Err(RustyJSError::TypeError(format!(
                            "Invalid referrerPolicy: {}",
                            policy
                        )))
                    }
                };
            }

            // Integrity
            if let Ok(integrity) = init.get::<_, String>("integrity") {
                request.integrity = integrity;
            }

            // Keepalive
            if let Ok(keepalive) = init.get::<_, bool>("keepalive") {
                request.keepalive = keepalive;
            }

            // Signal
            if let Ok(signal) = init.get::<_, JSObject>("signal") {
                request.signal = Some(signal);
            }
        }

        Ok(request)
    }

    #[js_method(getter)]
    pub fn method(&self) -> String {
        self.method.as_str().to_string()
    }

    #[js_method(getter)]
    pub fn url(&self) -> String {
        self.url_string.clone()
    }

    #[js_method(getter)]
    pub fn headers(&self) -> Headers {
        self.headers.clone()
    }

    #[js_method(getter)]
    pub fn destination(&self) -> String {
        // TODO: Implement destination getter
        "".to_string()
    }

    #[js_method(getter)]
    pub fn referrer(&self) -> String {
        self.referrer.clone()
    }

    #[js_method(getter, rename = "referrerPolicy")]
    pub fn referrer_policy(&self) -> String {
        match self.referrer_policy {
            ReferrerPolicy::NoReferrer => "no-referrer",
            ReferrerPolicy::NoReferrerWhenDowngrade => "no-referrer-when-downgrade",
            ReferrerPolicy::Origin => "origin",
            ReferrerPolicy::OriginWhenCrossOrigin => "origin-when-cross-origin",
            ReferrerPolicy::SameOrigin => "same-origin",
            ReferrerPolicy::StrictOrigin => "strict-origin",
            ReferrerPolicy::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin",
            ReferrerPolicy::UnsafeUrl => "unsafe-url",
        }
        .to_string()
    }

    #[js_method(getter)]
    pub fn mode(&self) -> String {
        match self.mode {
            RequestMode::Cors => "cors",
            RequestMode::NoCors => "no-cors",
            RequestMode::SameOrigin => "same-origin",
            RequestMode::Navigate => "navigate",
        }
        .to_string()
    }

    #[js_method(getter)]
    pub fn credentials(&self) -> String {
        match self.credentials {
            RequestCredentials::Omit => "omit",
            RequestCredentials::SameOrigin => "same-origin",
            RequestCredentials::Include => "include",
        }
        .to_string()
    }

    #[js_method(getter)]
    pub fn cache(&self) -> String {
        match self.cache {
            RequestCache::Default => "default",
            RequestCache::NoStore => "no-store",
            RequestCache::Reload => "reload",
            RequestCache::NoCache => "no-cache",
            RequestCache::ForceCache => "force-cache",
            RequestCache::OnlyIfCached => "only-if-cached",
        }
        .to_string()
    }

    #[js_method(getter)]
    pub fn redirect(&self) -> String {
        match self.redirect {
            RequestRedirect::Follow => "follow",
            RequestRedirect::Error => "error",
            RequestRedirect::Manual => "manual",
        }
        .to_string()
    }

    #[js_method(getter)]
    pub fn integrity(&self) -> String {
        self.integrity.clone()
    }

    #[js_method(getter)]
    pub fn keepalive(&self) -> bool {
        self.keepalive
    }

    #[js_method(getter, rename = "isReloadNavigation")]
    pub fn is_reload_navigation(&self) -> bool {
        self.is_reload_navigation
    }

    #[js_method(getter, rename = "isHistoryNavigation")]
    pub fn is_history_navigation(&self) -> bool {
        self.is_history_navigation
    }

    #[js_method(getter)]
    pub fn signal(&self) -> Option<JSObject> {
        self.signal.clone()
    }

    #[js_method(getter)]
    pub fn body(&self) -> Option<Blob> {
        self.body.clone()
    }

    #[js_method]
    pub fn clone(&self) -> Self {
        Self {
            method: self.method.clone(),
            url: self.url.clone(),
            url_string: self.url_string.clone(),
            headers: self.headers.clone(),
            body: self.body.clone(),
            mode: self.mode.clone(),
            credentials: self.credentials.clone(),
            cache: self.cache.clone(),
            redirect: self.redirect.clone(),
            referrer: self.referrer.clone(),
            referrer_policy: self.referrer_policy.clone(),
            integrity: self.integrity.clone(),
            keepalive: self.keepalive,
            is_reload_navigation: self.is_reload_navigation,
            is_history_navigation: self.is_history_navigation,
            signal: self.signal.clone(),
        }
    }

    #[js_method]
    pub async fn text(&self) -> JSResult<String> {
        if let Some(blob) = &self.body {
            blob.text().await
        } else {
            Ok(String::new())
        }
    }

    #[js_method]
    pub async fn json(&self, ctx: JSContext) -> JSResult<JSValue> {
        let text = self.text().await?;
        ctx.eval(Source::from_bytes(text.as_bytes()))
    }

    #[js_method(rename = "arrayBuffer")]
    pub async fn array_buffer(&self, ctx: JSContext) -> JSResult<JSArrayBuffer<u8>> {
        if let Some(blob) = &self.body {
            blob.array_buffer(ctx).await
        } else {
            JSArrayBuffer::from_bytes(&ctx, &[])
        }
    }

    #[js_method(rename = "formData")]
    pub async fn form_data(&self) -> JSResult<JSObject> {
        // TODO: Implement form data parsing
        Err(RustyJSError::TypeError("Not implemented".to_string()))
    }
}

impl Default for Request {
    fn default() -> Self {
        let url_str = "about:blank".to_string();
        Self {
            method: Method::GET,
            url: Uri::from_static("about:blank"),
            url_string: url_str,
            headers: Headers::default(),
            body: None,
            mode: RequestMode::default(),
            credentials: RequestCredentials::default(),
            cache: RequestCache::default(),
            redirect: RequestRedirect::default(),
            referrer: String::new(),
            referrer_policy: ReferrerPolicy::default(),
            integrity: String::new(),
            keepalive: false,
            is_reload_navigation: false,
            is_history_navigation: false,
            signal: None,
        }
    }
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<Request>();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyjs_test::*;

    #[test]
    fn test_request() {
        async_run!(|ctx: JSContext| async move {
            ctx.global().set(
                "print",
                JSFunc::new(&ctx, |msg: String| println!("JS: {}", msg)),
            );

            // Register console.log for better test output
            let source = Source::from_bytes(
                r#"
                const console = {
                    log: function(...args) {
                        print(args.join(' '))
                    }
                }
                "#,
            );
            let _ = ctx.eval::<()>(source);

            ctx.eval::<()>(Source::from_bytes(
                r#"
                class TextDecoder {
                    decode(arr) {
                        let str = '';
                        const len = arr.length;
                        let i = 0;

                        while (i < len) {
                            let charCode;

                            if (arr[i] < 0x80) {
                                charCode = arr[i++];
                            } else if (arr[i] < 0xE0) {
                                charCode = ((arr[i++] & 0x1F) << 6) |
                                         (arr[i++] & 0x3F);
                            } else {
                                charCode = ((arr[i++] & 0x0F) << 12) |
                                         ((arr[i++] & 0x3F) << 6) |
                                         (arr[i++] & 0x3F);
                            }

                            str += String.fromCharCode(charCode);
                        }

                        return str;
                    }
                }
                "#,
            ))
            .unwrap();

            // Initialize Blob first
            crate::blob::init(&ctx).unwrap();
            // Then initialize Headers
            crate::header::init(&ctx).unwrap();
            // Finally initialize Request
            init(&ctx).unwrap();

            let source = Source::from_path("tests/request.js").await.unwrap();
            let obj: JSObject = ctx.eval_async(source).await?;

            let total: i32 = obj.get("total").unwrap();
            let passed: i32 = obj.get("passed").unwrap();
            let success: bool = obj.get("success").unwrap();

            if !success {
                let failed: JSArray = obj.get("failed").unwrap();
                let error_messages: Vec<String> = failed.iter().collect::<JSResult<_>>()?;
                panic!(
                    "Request tests failed:\nPassed {}/{}\nFailures:\n{}",
                    passed,
                    total,
                    error_messages.join("\n")
                );
            }
            Ok(())
        });
    }
}
