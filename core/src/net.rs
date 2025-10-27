use bytes::Bytes;
use http::header;
use http::{HeaderValue, Request as HttpRequest};
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use std::io::Error;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread;
use tokio::fs;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::runtime::Builder;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, timeout};

pub enum HttpBody {
    Empty,
    Small(Bytes),
    Stream(mpsc::Receiver<Result<Bytes, String>>),
}

pub struct HttpResponse {
    pub status: http::StatusCode,
    pub headers: http::HeaderMap,
    pub body: HttpBody,
}

pub struct HttpJob {
    pub request: HttpRequest<BoxBody<Bytes, Error>>,
    pub small_threshold: usize,
    pub resp_tx: oneshot::Sender<Result<HttpResponse, String>>,
    pub abort_rx: Option<oneshot::Receiver<()>>,
}

enum NetCommand {
    Http(HttpJob),
    DownloadToFile {
        url: String,
        dest: PathBuf,
        abort_rx: Option<oneshot::Receiver<()>>,
        callback: Box<dyn FnOnce(Result<(), String>) + Send + 'static>,
    },
    Shutdown(oneshot::Sender<()>),
}

struct NetRuntime {
    tx: mpsc::Sender<NetCommand>,
    join: Option<std::thread::JoinHandle<()>>,
}

static RUNTIME_SLOT: OnceLock<Mutex<Option<NetRuntime>>> = OnceLock::new();
static USER_AGENT_SLOT: OnceLock<Mutex<String>> = OnceLock::new();
const DEFAULT_USER_AGENT: &str = concat!("RongJS/", env!("CARGO_PKG_VERSION"));

pub fn set_user_agent(ua: impl Into<String>) -> Result<(), String> {
    let ua_string = ua.into();
    HeaderValue::from_str(&ua_string).map_err(|e| format!("invalid user agent header: {}", e))?;
    let slot = USER_AGENT_SLOT.get_or_init(|| Mutex::new(DEFAULT_USER_AGENT.to_string()));
    let mut guard = slot.lock().unwrap();
    *guard = ua_string;
    Ok(())
}

pub fn get_user_agent() -> String {
    USER_AGENT_SLOT
        .get_or_init(|| Mutex::new(DEFAULT_USER_AGENT.to_string()))
        .lock()
        .unwrap()
        .clone()
}

fn runtime_slot() -> &'static Mutex<Option<NetRuntime>> {
    RUNTIME_SLOT.get_or_init(|| Mutex::new(None))
}

pub fn start_net_runtime(worker_threads: usize) {
    let slot = runtime_slot();
    let mut guard = slot.lock().unwrap();
    if guard.is_some() {
        return;
    }

    let (tx, mut rx) = mpsc::channel::<NetCommand>(256);
    let rt = Builder::new_multi_thread()
        .worker_threads(worker_threads.max(1))
        .enable_all()
        .build()
        .expect("failed to build net runtime");

    let join = thread::Builder::new()
        .name("rong-net".to_string())
        .spawn(move || {
            rt.block_on(async move {
                let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
                let https = HttpsConnectorBuilder::new()
                    .with_webpki_roots()
                    .https_or_http()
                    .enable_http1()
                    .build();
                let client: Client<_, BoxBody<Bytes, Error>> =
                    Client::builder(hyper_util::rt::TokioExecutor::new()).build(https);

                while let Some(cmd) = rx.recv().await {
                    match cmd {
                        NetCommand::Http(msg) => {
                            let client = client.clone();
                            tokio::task::spawn(async move {
                                process_request(client, msg).await;
                            });
                        }
                        NetCommand::DownloadToFile {
                            url,
                            dest,
                            abort_rx,
                            callback,
                        } => {
                            tokio::task::spawn(async move {
                                let res = download_to_file(&url, Path::new(&dest), abort_rx).await;
                                callback(res);
                            });
                        }
                        NetCommand::Shutdown(done_tx) => {
                            let _ = done_tx.send(());
                            break;
                        }
                    }
                }
            });
        })
        .expect("failed to spawn net runtime thread");

    *guard = Some(NetRuntime {
        tx,
        join: Some(join),
    });
}

pub async fn send_request(
    request: HttpRequest<BoxBody<Bytes, Error>>,
    small_threshold: usize,
    abort_rx: Option<oneshot::Receiver<()>>,
) -> Result<HttpResponse, String> {
    let tx = {
        let guard = runtime_slot().lock().unwrap();
        if let Some(rt) = guard.as_ref() {
            rt.tx.clone()
        } else {
            return Err("net runtime not started".to_string());
        }
    };
    let (resp_tx, resp_rx) = oneshot::channel();
    let msg = HttpJob {
        request,
        small_threshold,
        resp_tx,
        abort_rx,
    };
    tx.send(NetCommand::Http(msg))
        .await
        .map_err(|e| format!("net service down: {}", e))?;
    resp_rx
        .await
        .map_err(|e| format!("net resp dropped: {}", e))?
}

async fn process_request(
    client: Client<
        hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
        BoxBody<Bytes, Error>,
    >,
    mut msg: HttpJob,
) {
    let req = msg.request;
    let small = msg.small_threshold;
    // Conservative per-frame timeout to avoid indefinite stalls
    const READ_FRAME_TIMEOUT: Duration = Duration::from_secs(120);

    // Support cancellation during the request phase without consuming the abort for the body phase
    let mut abort_for_request = msg.abort_rx.as_mut();
    let resp = if let Some(abort_rx) = &mut abort_for_request {
        tokio::select! {
            res = client.request(req) => {
                match res {
                    Ok(r) => r,
                    Err(e) => { let _ = msg.resp_tx.send(Err(format!("request failed: {}", e))); return; }
                }
            }
            _ = abort_rx => {
                let _ = msg.resp_tx.send(Err("aborted".to_string()));
                return;
            }
        }
    } else {
        match client.request(req).await {
            Ok(r) => r,
            Err(e) => {
                let _ = msg.resp_tx.send(Err(format!("request failed: {}", e)));
                return;
            }
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
        let has_abort = msg.abort_rx.is_some();
        loop {
            if has_abort {
                tokio::select! {
                    maybe = timeout(READ_FRAME_TIMEOUT, body.frame()) => {
                        match maybe {
                            Ok(Some(Ok(frame))) => {
                                if let Some(data) = frame.data_ref() { buf.extend_from_slice(data); }
                                if buf.len() > small { let _ = msg.resp_tx.send(Err("body exceeded small threshold".to_string())); return; }
                            }
                            Ok(Some(Err(e))) => { let _ = msg.resp_tx.send(Err(format!("read frame: {}", e))); return; }
                            Ok(None) => break,
                            Err(_) => { let _ = msg.resp_tx.send(Err("read timeout".to_string())); return; }
                        }
                    }
                    _ = async { if let Some(rx) = &mut msg.abort_rx { let _ = rx.await; } } => {
                        let _ = msg.resp_tx.send(Err("aborted".to_string()));
                        return;
                    }
                }
            } else {
                match timeout(READ_FRAME_TIMEOUT, body.frame()).await {
                    Ok(Some(Ok(frame))) => {
                        if let Some(data) = frame.data_ref() {
                            buf.extend_from_slice(data);
                        }
                        if buf.len() > small {
                            let _ = msg
                                .resp_tx
                                .send(Err("body exceeded small threshold".to_string()));
                            return;
                        }
                    }
                    Ok(Some(Err(e))) => {
                        let _ = msg.resp_tx.send(Err(format!("read frame: {}", e)));
                        return;
                    }
                    Ok(None) => break,
                    Err(_) => {
                        let _ = msg.resp_tx.send(Err("read timeout".to_string()));
                        return;
                    }
                }
            }
        }
        let _ = msg.resp_tx.send(Ok(HttpResponse {
            status: parts.status,
            headers: parts.headers,
            body: HttpBody::Small(Bytes::from(buf)),
        }));
        return;
    }

    let (tx, rx) = mpsc::channel::<Result<Bytes, String>>(8);
    let mut abort = msg.abort_rx.take();
    tokio::task::spawn(async move {
        let mut body = body;
        let has_abort = abort.is_some();
        loop {
            if has_abort {
                tokio::select! {
                    maybe = timeout(READ_FRAME_TIMEOUT, body.frame()) => {
                        match maybe {
                            Ok(Some(Ok(frame))) => {
                                if let Ok(data) = frame.into_data() { if tx.send(Ok(data)).await.is_err() { break; } }
                            }
                            Ok(Some(Err(e))) => { let _ = tx.send(Err(format!("read frame: {}", e))).await; break; }
                            Ok(None) => break,
                            Err(_) => { let _ = tx.send(Err("read timeout".to_string())).await; break; }
                        }
                    }
                    _ = async { if let Some(rx) = &mut abort { let _ = rx.await; } } => { let _ = tx.send(Err("aborted".to_string())).await; drop(tx); break; }
                }
            } else {
                match timeout(READ_FRAME_TIMEOUT, body.frame()).await {
                    Ok(Some(Ok(frame))) => {
                        if let Ok(data) = frame.into_data() {
                            if tx.send(Ok(data)).await.is_err() {
                                break;
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
    });
    let _ = msg.resp_tx.send(Ok(HttpResponse {
        status: parts.status,
        headers: parts.headers,
        body: HttpBody::Stream(rx),
    }));
}

pub fn stop_net_runtime() {
    let slot = runtime_slot();
    let runtime = {
        let mut guard = slot.lock().unwrap();
        guard.take()
    };
    if let Some(mut rt) = runtime {
        let (done_tx, done_rx) = oneshot::channel();
        let _ = futures::executor::block_on(rt.tx.send(NetCommand::Shutdown(done_tx)));
        let _ = futures::executor::block_on(done_rx);
        if let Some(handle) = rt.join.take() {
            let _ = handle.join();
        }
    }
}

/// Request a download and return immediately, invoking `on_complete` on the
/// net runtime when finished.
pub fn request_download_to_file(
    url: impl Into<String>,
    dest: impl Into<PathBuf>,
    abort_rx: Option<oneshot::Receiver<()>>,
    on_complete: impl FnOnce(Result<(), String>) + Send + 'static,
) -> Result<(), String> {
    let tx = {
        let guard = runtime_slot().lock().unwrap();
        if let Some(rt) = guard.as_ref() {
            rt.tx.clone()
        } else {
            on_complete(Err("net runtime not started".to_string()));
            return Err("net runtime not started".to_string());
        }
    };
    let cmd = NetCommand::DownloadToFile {
        url: url.into(),
        dest: dest.into(),
        abort_rx,
        callback: Box::new(on_complete),
    };
    futures::executor::block_on(tx.send(cmd)).map_err(|e| format!("net service down: {}", e))
}

/// Download a URL directly to a file path using the net runtime.
///
/// - Issues a GET request and streams the body to `dest`.
/// - Uses a small buffer threshold to materialize tiny responses directly.
/// - Returns error if the HTTP status is not successful or IO fails.
async fn download_to_file(
    url: &str,
    dest: &Path,
    abort_rx: Option<tokio::sync::oneshot::Receiver<()>>,
) -> Result<(), String> {
    // Build minimal GET request
    let mut builder = HttpRequest::builder()
        .method("GET")
        .uri(url)
        .header(header::ACCEPT, "*/*");
    if let Some(headers) = builder.headers_mut() {
        let ua = get_user_agent();
        let header_value =
            HeaderValue::from_str(&ua).map_err(|e| format!("invalid user agent header: {}", e))?;
        headers.insert(header::USER_AGENT, header_value);
    }
    let empty = Full::new(Bytes::new()).map_err(|e| match e {}).boxed();
    let request = builder
        .body(empty)
        .map_err(|e| format!("build request: {}", e))?;

    let small_threshold = 64 * 1024usize;
    let resp = send_request(request, small_threshold, abort_rx).await?;
    if !resp.status.is_success() {
        return Err(format!("http status: {}", resp.status));
    }

    if let HttpBody::Small(buf) = &resp.body {
        if let Some(parent) = dest.parent() {
            if let Err(e) = fs::create_dir_all(parent).await {
                return Err(format!("create dir: {}", e));
            }
        }
        fs::write(dest, buf)
            .await
            .map_err(|e| format!("write: {}", e))?;
        return Ok(());
    }

    if let HttpBody::Stream(mut rx) = resp.body {
        if let Some(parent) = dest.parent() {
            if let Err(e) = fs::create_dir_all(parent).await {
                return Err(format!("create dir: {}", e));
            }
        }
        let file = fs::File::create(dest)
            .await
            .map_err(|e| format!("open: {}", e))?;
        let mut writer = BufWriter::new(file);
        while let Some(chunk_res) = rx.recv().await {
            let chunk = chunk_res.map_err(|e| e)?;
            writer
                .write_all(chunk.as_ref())
                .await
                .map_err(|e| format!("write chunk: {}", e))?;
        }
        writer.flush().await.map_err(|e| format!("flush: {}", e))?;
        return Ok(());
    }

    // No body: create empty file
    if let Some(parent) = dest.parent() {
        if let Err(e) = fs::create_dir_all(parent).await {
            return Err(format!("create dir: {}", e));
        }
    }
    fs::File::create(dest)
        .await
        .map_err(|e| format!("create: {}", e))?;
    Ok(())
}
