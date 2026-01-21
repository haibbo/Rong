use bytes::Bytes;
use http::Request as HttpRequest;
use http::header;
use http::{HeaderValue, header::HeaderName};
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use std::io::Error;
use std::sync::OnceLock;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, timeout};

pub const DEFAULT_BLOCKING_BODY_LIMIT: usize = 512 * 1024;
pub const DEFAULT_STREAM_COALESCE_TARGET: usize = 512 * 1024;
const MIN_STREAM_COALESCE_TARGET: usize = 4 * 1024;
const STREAM_CHAN_CAP: usize = 256;

type HttpClient = Client<
    hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
    BoxBody<Bytes, Error>,
>;

static CLIENT: OnceLock<HttpClient> = OnceLock::new();

fn ensure_bg_started() -> Result<(), String> {
    if rong::bg::is_started() {
        return Ok(());
    }
    Err("background task manager not started (call `Rong::builder().build()` or `rong::bg::start(...)` first)".to_string())
}

fn client() -> &'static HttpClient {
    CLIENT.get_or_init(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
        let https = HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1()
            .build();
        Client::builder(hyper_util::rt::TokioExecutor::new()).build(https)
    })
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
    let mut builder = HttpRequest::builder()
        .method("POST")
        .uri(url)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json");

    if let Some(h) = builder.headers_mut() {
        let ua = rong::get_user_agent();
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

    let response = send_request(request, DEFAULT_BLOCKING_BODY_LIMIT, None).await?;
    let status = response.status;
    let bytes = collect_body_bytes(response.body).await?;
    Ok((status, bytes))
}

pub async fn send_request(
    request: HttpRequest<BoxBody<Bytes, Error>>,
    small_threshold: usize,
    abort_rx: Option<oneshot::Receiver<()>>,
) -> Result<HttpResponse, String> {
    send_request_with_coalesce(
        request,
        small_threshold,
        abort_rx,
        DEFAULT_STREAM_COALESCE_TARGET,
    )
    .await
}

pub async fn send_request_with_coalesce(
    request: HttpRequest<BoxBody<Bytes, Error>>,
    small_threshold: usize,
    abort_rx: Option<oneshot::Receiver<()>>,
    stream_coalesce_target: usize,
) -> Result<HttpResponse, String> {
    ensure_bg_started()?;
    let client = client().clone();
    let join = rong::bg::spawn(async move {
        process_request(
            client,
            request,
            small_threshold,
            stream_coalesce_target,
            abort_rx,
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
) -> Result<HttpResponse, String> {
    const READ_FRAME_TIMEOUT: Duration = Duration::from_secs(120);

    let resp = if let Some(rx) = abort_rx.as_mut() {
        tokio::select! {
            res = client.request(req) => match res {
                Ok(r) => r,
                Err(e) => return Err(format!("request failed: {}", e)),
            },
            _ = rx => return Err("aborted".to_string()),
        }
    } else {
        match client.request(req).await {
            Ok(r) => r,
            Err(e) => return Err(format!("request failed: {}", e)),
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
    let coalesce_target = stream_coalesce_target.max(MIN_STREAM_COALESCE_TARGET);
    let tx_monitor = tx.clone();
    let stream_task = tokio::task::spawn(async move {
        let mut body = body;
        let mut buf: bytes::BytesMut = bytes::BytesMut::with_capacity(coalesce_target);
        let has_abort = abort.is_some();
        let mut aborted = false;
        loop {
            if has_abort {
                tokio::select! {
                    maybe = timeout(READ_FRAME_TIMEOUT, body.frame()) => {
                        match maybe {
                            Ok(Some(Ok(frame))) => {
                                if let Ok(data) = frame.into_data() {
                                    if buf.is_empty() && data.len() >= coalesce_target {
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
                            if buf.is_empty() && data.len() >= coalesce_target {
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
