use bytes::Bytes;
use http::Request as HttpRequest;
use http::Uri;
use http::header;
use http_body_util::{BodyExt, combinators::BoxBody};
use hyper_http_proxy::{Intercept, Proxy, ProxyConnector};
#[cfg(any(feature = "tls-aws-lc", feature = "tls-ring"))]
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use std::io::Error;
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

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RequestTimeouts {
    pub request_timeout: Option<Duration>,
    pub connect_timeout: Option<Duration>,
}

#[cfg(all(feature = "tls-aws-lc", feature = "tls-ring"))]
compile_error!("Enable only one TLS backend feature for rong_rt: `tls-aws-lc` or `tls-ring`.");

#[cfg(not(any(feature = "tls-aws-lc", feature = "tls-ring")))]
compile_error!(
    "rong_rt requires an explicit TLS backend. Enable exactly one of `tls-aws-lc` or `tls-ring` from your top-level crate."
);

static CLIENT: OnceLock<Mutex<Option<CachedClient>>> = OnceLock::new();
static PROXY_CONFIG: OnceLock<RwLock<Option<ProxyConfig>>> = OnceLock::new();
#[cfg(test)]
static TEST_GUARD: OnceLock<Mutex<()>> = OnceLock::new();

async fn forward_or_buffer_chunk(
    tx: &mpsc::Sender<Result<Bytes, String>>,
    buf: &mut bytes::BytesMut,
    data: Bytes,
    coalesce_target: usize,
) -> bool {
    if coalesce_target == 0 || (buf.is_empty() && data.len() >= coalesce_target) {
        return tx.send(Ok(data)).await.is_ok();
    }

    buf.extend_from_slice(&data);
    if buf.len() >= coalesce_target {
        let out = buf.split().freeze();
        return tx.send(Ok(out)).await.is_ok();
    }

    true
}

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

#[cfg(test)]
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    TEST_GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test guard lock")
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

#[cfg(feature = "tls-aws-lc")]
fn tls_provider() -> rustls::crypto::CryptoProvider {
    rustls::crypto::aws_lc_rs::default_provider()
}

#[cfg(feature = "tls-ring")]
fn tls_provider() -> rustls::crypto::CryptoProvider {
    rustls::crypto::ring::default_provider()
}

#[cfg(any(feature = "tls-aws-lc", feature = "tls-ring"))]
fn build_client(proxy: Option<ProxyConfig>, connect_timeout: Option<Duration>) -> HttpClient {
    let provider = tls_provider();

    let _ = provider.clone().install_default();

    let mut connector = HttpConnector::new();
    // Required when using wrap_connector and https URIs.
    connector.enforce_http(false);
    connector.set_connect_timeout(connect_timeout);

    let mut proxy_connector = ProxyConnector::unsecured(connector);
    if let Some(proxy_config) = proxy {
        let mut proxy = build_proxy(proxy_config);
        // Use CONNECT for both HTTP/HTTPS to keep request path handling simple.
        proxy.force_connect();
        proxy_connector.add_proxy(proxy);
    }

    let https = HttpsConnectorBuilder::new()
        .with_provider_and_webpki_roots(provider)
        .expect("failed to configure TLS root store")
        .https_or_http()
        .enable_http1()
        .wrap_connector(proxy_connector);

    Client::builder(hyper_util::rt::TokioExecutor::new()).build(https)
}

#[cfg(not(any(feature = "tls-aws-lc", feature = "tls-ring")))]
fn build_client(_proxy: Option<ProxyConfig>, _connect_timeout: Option<Duration>) -> HttpClient {
    unreachable!("compile_error should require an explicit TLS backend before build_client()")
}

fn build_proxy(proxy_config: ProxyConfig) -> Proxy {
    Proxy::new(Intercept::All, proxy_config.uri)
}

fn client(timeouts: RequestTimeouts) -> Result<HttpClient, String> {
    let proxy = current_proxy();
    let connect_timeout = timeouts.connect_timeout;

    if let Ok(slot) = client_cache().lock()
        && let Some(cached) = slot.as_ref()
        && cached.proxy == proxy
        && connect_timeout.is_none()
    {
        return Ok(cached.client.clone());
    }

    let built = build_client(proxy.clone(), connect_timeout);
    if connect_timeout.is_some() {
        return Ok(built);
    }

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

pub async fn send_request_with_timeout(
    request: HttpRequest<BoxBody<Bytes, Error>>,
    small_threshold: usize,
    abort_rx: Option<oneshot::Receiver<()>>,
    timeouts: RequestTimeouts,
) -> Result<HttpResponse, String> {
    send_request_with_coalesce(
        request,
        small_threshold,
        abort_rx,
        DEFAULT_STREAM_COALESCE_TARGET,
        timeouts,
    )
    .await
}

pub async fn send_request_with_coalesce(
    request: HttpRequest<BoxBody<Bytes, Error>>,
    small_threshold: usize,
    abort_rx: Option<oneshot::Receiver<()>>,
    stream_coalesce_target: usize,
    timeouts: RequestTimeouts,
) -> Result<HttpResponse, String> {
    let request_timeout = timeouts.request_timeout.unwrap_or(DEFAULT_REQUEST_TIMEOUT);
    let client = client(timeouts)?;
    let join = crate::RongExecutor::global().spawn(async move {
        process_request(
            client,
            request,
            small_threshold,
            stream_coalesce_target,
            abort_rx,
            request_timeout,
        )
        .await
    });

    join.await
        .map_err(|e| format!("user task panicked or runtime dropped: {}", e))?
}

async fn process_request(
    client: HttpClient,
    req: HttpRequest<BoxBody<Bytes, Error>>,
    small: usize,
    stream_coalesce_target: usize,
    mut abort_rx: Option<oneshot::Receiver<()>>,
    request_timeout: Duration,
) -> Result<HttpResponse, String> {
    const READ_FRAME_TIMEOUT: Duration = Duration::from_secs(120);

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
                                if let Ok(data) = frame.into_data()
                                    && !forward_or_buffer_chunk(&tx, &mut buf, data, coalesce_target).await
                                {
                                    break;
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
                        if let Ok(data) = frame.into_data()
                            && !forward_or_buffer_chunk(&tx, &mut buf, data, coalesce_target).await
                        {
                            break;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_connect_timeout_does_not_populate_shared_cache() {
        let _guard = test_guard();
        invalidate_client_cache();
        clear_proxy();

        let _ = client(RequestTimeouts {
            connect_timeout: Some(Duration::from_secs(1)),
            ..Default::default()
        })
        .expect("custom-timeout client");
        assert!(client_cache().lock().expect("cache lock").is_none());

        let _ = client(RequestTimeouts::default()).expect("default client");
        assert!(client_cache().lock().expect("cache lock").is_some());
    }

    #[test]
    fn custom_connect_timeout_keeps_shared_cache_intact() {
        let _guard = test_guard();
        invalidate_client_cache();
        clear_proxy();

        let _ = client(RequestTimeouts::default()).expect("default client");
        let had_cached_client_before = client_cache()
            .lock()
            .expect("cache lock")
            .as_ref()
            .is_some();

        let _ = client(RequestTimeouts {
            connect_timeout: Some(Duration::from_secs(1)),
            ..Default::default()
        })
        .expect("custom-timeout client");

        let had_cached_client_after = client_cache()
            .lock()
            .expect("cache lock")
            .as_ref()
            .is_some();

        assert!(had_cached_client_before);
        assert!(had_cached_client_after);
    }

    #[test]
    fn proxy_url_supports_basic_auth() {
        let _guard = test_guard();
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
        let _guard = test_guard();
        let uri = parse_proxy_uri("http://127.0.0.1:8080").expect("valid proxy uri");
        let proxy = build_proxy(ProxyConfig { uri });
        assert!(proxy.headers().get("authorization").is_none());
        assert!(proxy.headers().get("proxy-authorization").is_none());
    }

    #[test]
    fn proxy_only_supports_http_scheme() {
        let _guard = test_guard();
        let err = parse_proxy_uri("https://127.0.0.1:8080").expect_err("must reject https");
        assert!(err.contains("only http:// is supported"));
    }
}
