use bytes::Bytes;
use http::Request as HttpRequest;
use http::header;
use http::{HeaderValue, Uri, header::HeaderName};
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper_http_proxy::{Intercept, Proxy, ProxyConnector};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use std::io::Error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, RwLock};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, timeout};

pub const DEFAULT_BLOCKING_BODY_LIMIT: usize = 512 * 1024;
pub const DEFAULT_STREAM_COALESCE_TARGET: usize = 512 * 1024;
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MIN_STREAM_COALESCE_TARGET: usize = 4 * 1024;
const STREAM_CHAN_CAP: usize = 256;

type HttpClient =
    Client<hyper_rustls::HttpsConnector<ProxyConnector<HttpConnector>>, BoxBody<Bytes, Error>>;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProxyConfig {
    uri: Uri,
}

#[derive(Clone)]
struct CachedClient {
    proxy: Option<ProxyConfig>,
    client: HttpClient,
}

#[cfg(all(feature = "tls-aws-lc", feature = "tls-ring"))]
compile_error!("Enable only one TLS backend feature: `tls-aws-lc` or `tls-ring`.");

#[cfg(not(any(feature = "tls-aws-lc", feature = "tls-ring")))]
compile_error!("One TLS backend feature is required: enable `tls-aws-lc` or `tls-ring`.");

static CLIENT: OnceLock<Mutex<Option<CachedClient>>> = OnceLock::new();
static PROXY_CONFIG: OnceLock<RwLock<Option<ProxyConfig>>> = OnceLock::new();
static REQUEST_TIMEOUT_MS: AtomicU64 = AtomicU64::new(DEFAULT_REQUEST_TIMEOUT.as_millis() as u64);

fn client_cache() -> &'static Mutex<Option<CachedClient>> {
    CLIENT.get_or_init(|| Mutex::new(None))
}

fn proxy_config_store() -> &'static RwLock<Option<ProxyConfig>> {
    PROXY_CONFIG.get_or_init(|| RwLock::new(None))
}

fn invalidate_client_cache() {
    if let Ok(mut slot) = client_cache().lock() {
        *slot = None;
    }
}

fn current_proxy() -> Option<ProxyConfig> {
    proxy_config_store()
        .read()
        .ok()
        .and_then(|g| g.as_ref().cloned())
}

fn parse_proxy_uri(proxy_url: &str) -> Result<Uri, String> {
    let uri = proxy_url
        .parse::<Uri>()
        .map_err(|e| format!("invalid proxy URL: {}", e))?;

    if uri.scheme_str() != Some("http") {
        return Err("unsupported proxy URL scheme (only http:// is supported)".to_string());
    }

    uri.authority()
        .ok_or_else(|| "proxy URL must include host[:port]".to_string())?;

    if uri.host().is_none() {
        return Err("proxy URL must include host".to_string());
    }

    Ok(uri)
}

/// Configure global HTTP proxy (no auth, no env fallback).
/// Supported formats:
/// - `http://host:port`
/// - `http://username:password@host:port` (Basic proxy auth)
pub fn set_proxy(proxy_url: &str) -> Result<(), String> {
    let uri = parse_proxy_uri(proxy_url)?;
    {
        let mut proxy = proxy_config_store()
            .write()
            .map_err(|_| "proxy config lock poisoned".to_string())?;
        *proxy = Some(ProxyConfig { uri });
    }
    invalidate_client_cache();
    Ok(())
}

/// Clear global proxy configuration.
pub fn clear_proxy() {
    if let Ok(mut proxy) = proxy_config_store().write() {
        *proxy = None;
    }
    invalidate_client_cache();
}

/// Read current proxy URL.
pub fn get_proxy() -> Option<String> {
    current_proxy().map(|p| p.uri.to_string())
}

pub fn set_request_timeout(timeout: Duration) {
    // Keep timeout > 0; use default if caller passes zero.
    let millis = timeout.as_millis() as u64;
    REQUEST_TIMEOUT_MS.store(
        if millis == 0 {
            DEFAULT_REQUEST_TIMEOUT.as_millis() as u64
        } else {
            millis
        },
        Ordering::Relaxed,
    );
}

pub fn get_request_timeout() -> Duration {
    Duration::from_millis(REQUEST_TIMEOUT_MS.load(Ordering::Relaxed))
}

pub fn reset_request_timeout() {
    REQUEST_TIMEOUT_MS.store(
        DEFAULT_REQUEST_TIMEOUT.as_millis() as u64,
        Ordering::Relaxed,
    );
}

fn ensure_bg_started() -> Result<(), String> {
    if crate::is_started() {
        return Ok(());
    }
    Err("background task manager not started (call `Rong::builder().build()` or `crate::start(...)` first)".to_string())
}

fn build_client(proxy: Option<ProxyConfig>) -> HttpClient {
    #[cfg(feature = "tls-aws-lc")]
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    #[cfg(feature = "tls-ring")]
    let _ = rustls::crypto::ring::default_provider().install_default();

    let mut connector = HttpConnector::new();
    // Required when using wrap_connector and https URIs.
    connector.enforce_http(false);

    let mut proxy_connector = ProxyConnector::unsecured(connector);
    if let Some(proxy_config) = proxy {
        let mut proxy = build_proxy(proxy_config);
        // Use CONNECT for both HTTP/HTTPS to keep request path handling simple.
        proxy.force_connect();
        proxy_connector.add_proxy(proxy);
    }

    let https = HttpsConnectorBuilder::new()
        .with_webpki_roots()
        .https_or_http()
        .enable_http1()
        .wrap_connector(proxy_connector);

    Client::builder(hyper_util::rt::TokioExecutor::new()).build(https)
}

fn build_proxy(proxy_config: ProxyConfig) -> Proxy {
    Proxy::new(Intercept::All, proxy_config.uri)
}

fn client() -> Result<HttpClient, String> {
    let proxy = current_proxy();

    if let Ok(slot) = client_cache().lock()
        && let Some(cached) = slot.as_ref()
        && cached.proxy == proxy
    {
        return Ok(cached.client.clone());
    }

    let built = build_client(proxy.clone());
    let mut slot = client_cache()
        .lock()
        .map_err(|_| "client cache lock poisoned".to_string())?;
    *slot = Some(CachedClient {
        proxy,
        client: built.clone(),
    });
    Ok(built)
}

pub struct HttpResponse {
    pub status: http::StatusCode,
    pub headers: http::HeaderMap,
    pub body: HttpBody,
}

pub enum HttpBody {
    Empty,
    Small(Bytes),
    Stream(mpsc::Receiver<Result<Bytes, String>>),
}

pub async fn post_json(
    url: &str,
    body: &[u8],
    extra_headers: Option<&[(&str, &str)]>,
) -> Result<(http::StatusCode, Bytes), String> {
    post_json_inner(url, body, extra_headers, None).await
}

pub async fn post_json_with_timeout(
    url: &str,
    body: &[u8],
    extra_headers: Option<&[(&str, &str)]>,
    request_timeout: Duration,
) -> Result<(http::StatusCode, Bytes), String> {
    post_json_inner(url, body, extra_headers, Some(request_timeout)).await
}

async fn post_json_inner(
    url: &str,
    body: &[u8],
    extra_headers: Option<&[(&str, &str)]>,
    timeout_override: Option<Duration>,
) -> Result<(http::StatusCode, Bytes), String> {
    let mut builder = HttpRequest::builder()
        .method("POST")
        .uri(url)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json");

    if let Some(h) = builder.headers_mut() {
        let ua = crate::get_user_agent();
        let ua_val =
            HeaderValue::from_str(&ua).map_err(|e| format!("invalid user agent header: {}", e))?;
        h.insert(header::USER_AGENT, ua_val);

        if let Some(extras) = extra_headers {
            for (key, value) in extras {
                let name = HeaderName::from_bytes(key.as_bytes())
                    .map_err(|e| format!("invalid header name '{}': {}", key, e))?;
                let val = HeaderValue::from_str(value)
                    .map_err(|e| format!("invalid header '{}' value: {}", key, e))?;
                h.insert(name, val);
            }
        }
    }

    let body_bytes = Bytes::copy_from_slice(body);
    let request_body: BoxBody<Bytes, Error> = Full::new(body_bytes)
        .map_err(|_| Error::other("body error"))
        .boxed();

    let request = builder
        .body(request_body)
        .map_err(|e| format!("build request: {}", e))?;

    let response =
        send_request_with_timeout(request, DEFAULT_BLOCKING_BODY_LIMIT, None, timeout_override)
            .await?;
    let status = response.status;
    let bytes = collect_body_bytes(response.body).await?;
    Ok((status, bytes))
}

pub async fn send_request(
    request: HttpRequest<BoxBody<Bytes, Error>>,
    small_threshold: usize,
    abort_rx: Option<oneshot::Receiver<()>>,
) -> Result<HttpResponse, String> {
    send_request_with_timeout(request, small_threshold, abort_rx, None).await
}

pub async fn send_request_with_timeout(
    request: HttpRequest<BoxBody<Bytes, Error>>,
    small_threshold: usize,
    abort_rx: Option<oneshot::Receiver<()>>,
    timeout_override: Option<Duration>,
) -> Result<HttpResponse, String> {
    send_request_with_coalesce(
        request,
        small_threshold,
        abort_rx,
        DEFAULT_STREAM_COALESCE_TARGET,
        timeout_override,
    )
    .await
}

pub async fn send_request_with_coalesce(
    request: HttpRequest<BoxBody<Bytes, Error>>,
    small_threshold: usize,
    abort_rx: Option<oneshot::Receiver<()>>,
    stream_coalesce_target: usize,
    timeout_override: Option<Duration>,
) -> Result<HttpResponse, String> {
    ensure_bg_started()?;
    let client = client()?;
    let join = crate::spawn(async move {
        process_request(
            client,
            request,
            small_threshold,
            stream_coalesce_target,
            abort_rx,
            timeout_override,
        )
        .await
    })
    .map_err(|e| e.to_string())?;

    join.await
        .map_err(|e| format!("user task panicked or runtime dropped: {}", e))?
}

async fn process_request(
    client: HttpClient,
    req: HttpRequest<BoxBody<Bytes, Error>>,
    small: usize,
    stream_coalesce_target: usize,
    mut abort_rx: Option<oneshot::Receiver<()>>,
    timeout_override: Option<Duration>,
) -> Result<HttpResponse, String> {
    const READ_FRAME_TIMEOUT: Duration = Duration::from_secs(120);
    let request_timeout = timeout_override.unwrap_or_else(get_request_timeout);

    let resp = if let Some(rx) = abort_rx.as_mut() {
        tokio::select! {
            res = timeout(request_timeout, client.request(req)) => match res {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => return Err(format!("request failed: {}", e)),
                Err(_) => return Err("request timeout".to_string()),
            },
            _ = rx => return Err("aborted".to_string()),
        }
    } else {
        match timeout(request_timeout, client.request(req)).await {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => return Err(format!("request failed: {}", e)),
            Err(_) => return Err("request timeout".to_string()),
        }
    };
    let (parts, mut body) = resp.into_parts();

    let cl = parts
        .headers
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(0);

    if cl > 0 && cl <= small {
        let mut buf = Vec::with_capacity(cl);
        let has_abort = abort_rx.is_some();
        loop {
            if has_abort {
                tokio::select! {
                    maybe = timeout(READ_FRAME_TIMEOUT, body.frame()) => {
                        match maybe {
                            Ok(Some(Ok(frame))) => {
                                if let Some(data) = frame.data_ref() { buf.extend_from_slice(data); }
                                if buf.len() > small { return Err("body exceeded small threshold".to_string()); }
                            }
                            Ok(Some(Err(e))) => return Err(format!("read frame: {}", e)),
                            Ok(None) => break,
                            Err(_) => return Err("read timeout".to_string()),
                        }
                    }
                    _ = async { if let Some(rx) = abort_rx.as_mut() { let _ = rx.await; } } => return Err("aborted".to_string()),
                }
            } else {
                match timeout(READ_FRAME_TIMEOUT, body.frame()).await {
                    Ok(Some(Ok(frame))) => {
                        if let Some(data) = frame.data_ref() {
                            buf.extend_from_slice(data);
                        }
                        if buf.len() > small {
                            return Err("body exceeded small threshold".to_string());
                        }
                    }
                    Ok(Some(Err(e))) => return Err(format!("read frame: {}", e)),
                    Ok(None) => break,
                    Err(_) => return Err("read timeout".to_string()),
                }
            }
        }
        return Ok(HttpResponse {
            status: parts.status,
            headers: parts.headers,
            body: HttpBody::Small(Bytes::from(buf)),
        });
    }

    let (tx, rx) = mpsc::channel::<Result<Bytes, String>>(STREAM_CHAN_CAP);
    let mut abort = abort_rx.take();
    // `0` disables response-body coalescing and forwards frames as they arrive.
    let coalesce_target = if stream_coalesce_target == 0 {
        0
    } else {
        stream_coalesce_target.max(MIN_STREAM_COALESCE_TARGET)
    };
    let tx_monitor = tx.clone();
    let stream_task = tokio::task::spawn(async move {
        let mut body = body;
        let mut buf: bytes::BytesMut = if coalesce_target == 0 {
            bytes::BytesMut::new()
        } else {
            bytes::BytesMut::with_capacity(coalesce_target)
        };
        let has_abort = abort.is_some();
        let mut aborted = false;
        loop {
            if has_abort {
                tokio::select! {
                    maybe = timeout(READ_FRAME_TIMEOUT, body.frame()) => {
                        match maybe {
                            Ok(Some(Ok(frame))) => {
                                if let Ok(data) = frame.into_data() {
                                    if coalesce_target == 0 {
                                        if tx.send(Ok(data)).await.is_err() { break; }
                                    } else if buf.is_empty() && data.len() >= coalesce_target {
                                        if tx.send(Ok(data)).await.is_err() { break; }
                                    } else {
                                        buf.extend_from_slice(&data);
                                        if buf.len() >= coalesce_target {
                                            let out = buf.split().freeze();
                                            if tx.send(Ok(out)).await.is_err() { break; }
                                        }
                                    }
                                }
                            }
                            Ok(Some(Err(e))) => { let _ = tx.send(Err(format!("read frame: {}", e))).await; break; }
                            Ok(None) => break,
                            Err(_) => { let _ = tx.send(Err("read timeout".to_string())).await; break; }
                        }
                    }
                    _ = async { if let Some(rx) = &mut abort { let _ = rx.await; } } => { let _ = tx.send(Err("aborted".to_string())).await; aborted = true; break; }
                }
            } else {
                match timeout(READ_FRAME_TIMEOUT, body.frame()).await {
                    Ok(Some(Ok(frame))) => {
                        if let Ok(data) = frame.into_data() {
                            if coalesce_target == 0 {
                                if tx.send(Ok(data)).await.is_err() {
                                    break;
                                }
                            } else if buf.is_empty() && data.len() >= coalesce_target {
                                if tx.send(Ok(data)).await.is_err() {
                                    break;
                                }
                            } else {
                                buf.extend_from_slice(&data);
                                if buf.len() >= coalesce_target {
                                    let out = buf.split().freeze();
                                    if tx.send(Ok(out)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Ok(Some(Err(e))) => {
                        let _ = tx.send(Err(format!("read frame: {}", e))).await;
                        break;
                    }
                    Ok(None) => break,
                    Err(_) => {
                        let _ = tx.send(Err("read timeout".to_string())).await;
                        break;
                    }
                }
            }
        }
        if !aborted && !buf.is_empty() {
            let out = buf.split().freeze();
            let _ = tx.send(Ok(out)).await;
        }
    });
    tokio::task::spawn(async move {
        if let Err(join_err) = stream_task.await {
            if join_err.is_panic() {
                let _ = tx_monitor
                    .send(Err("stream task panicked".to_string()))
                    .await;
            } else if join_err.is_cancelled() {
                let _ = tx_monitor
                    .send(Err("stream task cancelled".to_string()))
                    .await;
            }
        }
    });

    Ok(HttpResponse {
        status: parts.status,
        headers: parts.headers,
        body: HttpBody::Stream(rx),
    })
}

async fn collect_body_bytes(body: HttpBody) -> Result<Bytes, String> {
    match body {
        HttpBody::Empty => Ok(Bytes::new()),
        HttpBody::Small(bytes) => Ok(bytes),
        HttpBody::Stream(mut rx) => {
            let mut buf = Vec::new();
            while let Some(chunk_res) = rx.recv().await {
                let chunk = chunk_res?;
                buf.extend_from_slice(&chunk);
            }
            Ok(Bytes::from(buf))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[test]
    fn proxy_url_supports_basic_auth() {
        let uri = parse_proxy_uri("http://bob:secret@127.0.0.1:8080").expect("valid proxy uri");
        let proxy = build_proxy(ProxyConfig { uri });
        let auth = proxy
            .headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let proxy_auth = proxy
            .headers()
            .get("proxy-authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        assert_eq!(auth, "Basic Ym9iOnNlY3JldA==");
        assert_eq!(proxy_auth, "Basic Ym9iOnNlY3JldA==");
    }

    #[test]
    fn proxy_url_without_auth_has_no_auth_headers() {
        let uri = parse_proxy_uri("http://127.0.0.1:8080").expect("valid proxy uri");
        let proxy = build_proxy(ProxyConfig { uri });
        assert!(proxy.headers().get("authorization").is_none());
        assert!(proxy.headers().get("proxy-authorization").is_none());
    }

    #[test]
    fn proxy_only_supports_http_scheme() {
        let err = parse_proxy_uri("https://127.0.0.1:8080").expect_err("must reject https");
        assert!(err.contains("only http:// is supported"));
    }

    fn ensure_started() {
        crate::start(1);
    }

    async fn spawn_echo_server() -> std::net::SocketAddr {
        use axum::Router;
        use axum::body::Bytes as AxumBytes;
        use axum::http::HeaderMap;
        use axum::response::IntoResponse;
        use axum::routing::post;

        async fn echo(headers: HeaderMap, body: AxumBytes) -> impl IntoResponse {
            (headers, body)
        }

        let app = Router::new().route("/echo", post(echo));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        addr
    }

    async fn spawn_slow_server(delay_ms: u64) -> std::net::SocketAddr {
        use axum::Router;
        use axum::routing::post;
        use std::sync::Arc;

        let delay = Arc::new(delay_ms);
        let app = Router::new().route(
            "/slow",
            post({
                let delay = delay.clone();
                move || {
                    let d = *delay;
                    async move {
                        sleep(Duration::from_millis(d)).await;
                        "ok"
                    }
                }
            }),
        );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        addr
    }

    #[test]
    fn post_json_with_timeout_succeeds() {
        ensure_started();
        let handle = crate::handle().unwrap();
        handle.block_on(async {
            let addr = spawn_echo_server().await;
            let url = format!("http://{}/echo", addr);
            let body = br#"{"hello":"world"}"#;
            let (status, bytes) = post_json_with_timeout(&url, body, None, Duration::from_secs(5))
                .await
                .expect("request should succeed");
            assert_eq!(status, http::StatusCode::OK);
            assert_eq!(bytes.as_ref(), body);
        });
    }

    #[test]
    fn post_json_with_timeout_expires() {
        ensure_started();
        let handle = crate::handle().unwrap();
        handle.block_on(async {
            let addr = spawn_slow_server(300).await;
            let url = format!("http://{}/slow", addr);
            let err = post_json_with_timeout(&url, b"{}", None, Duration::from_millis(10))
                .await
                .expect_err("should time out");
            assert!(
                err.contains("timeout"),
                "expected timeout error, got: {}",
                err
            );
        });
    }

    #[test]
    fn post_json_uses_global_timeout_by_default() {
        // Verify the original post_json still respects the global timeout.
        ensure_started();
        let handle = crate::handle().unwrap();
        handle.block_on(async {
            let addr = spawn_echo_server().await;
            let url = format!("http://{}/echo", addr);
            let (status, _) = post_json(&url, b"{}", None)
                .await
                .expect("request should succeed");
            assert_eq!(status, http::StatusCode::OK);
        });
    }
}
