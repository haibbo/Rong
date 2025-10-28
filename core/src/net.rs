use bytes::Bytes;
use http::header;
use http::{HeaderValue, Request as HttpRequest};
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use std::io::Error;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::thread;
use tokio::io::AsyncWriteExt;
use tokio::runtime::Builder;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, timeout};

/// Additional consumer for the HTTP body
pub trait BodySink: Send {
    /// Consume a body chunk. Returning `Err` stops further writes to the sink.
    fn write(&mut self, chunk: &[u8]) -> Result<(), String>;

    /// Called when the download completes (success or failure).
    fn close(&mut self, result: &Result<(), String>);
}

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
    Download {
        url: String,
        dest: PathBuf,
        abort_rx: Option<oneshot::Receiver<()>>,
        sink: Option<Box<dyn BodySink>>,
        completion: oneshot::Sender<Result<(), String>>,
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
                        NetCommand::Download {
                            url,
                            dest,
                            abort_rx,
                            sink,
                            completion,
                        } => {
                            tokio::task::spawn(async move {
                                let res = download_resource(&url, &dest, abort_rx, sink).await;
                                let _ = completion.send(res);
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

/// Start a download to a file on disk while optionally streaming chunks to an extra sink.
async fn download_resource(
    url: &str,
    dest: &PathBuf,
    abort_rx: Option<oneshot::Receiver<()>>,
    sink: Option<Box<dyn BodySink>>,
) -> Result<(), String> {
    let mut sink_opt = sink;

    if let Some(parent) = dest.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return finalize_sink(sink_opt, Err(format!("create dir: {}", e)));
        }
    }

    let temp_path = dest.with_extension("part");
    let mut file = match tokio::fs::File::create(&temp_path).await {
        Ok(f) => f,
        Err(e) => return finalize_sink(sink_opt, Err(format!("open: {}", e))),
    };

    let mut builder = HttpRequest::builder()
        .method("GET")
        .uri(url)
        .header(header::ACCEPT, "*/*");
    if let Some(headers) = builder.headers_mut() {
        let ua = get_user_agent();
        let header_value = match HeaderValue::from_str(&ua) {
            Ok(v) => v,
            Err(e) => {
                return finalize_sink(sink_opt, Err(format!("invalid user agent header: {}", e)));
            }
        };
        headers.insert(header::USER_AGENT, header_value);
    }
    let empty = Full::new(Bytes::new()).map_err(|e| match e {}).boxed();
    let request = match builder.body(empty) {
        Ok(req) => req,
        Err(e) => return finalize_sink(sink_opt, Err(format!("build request: {}", e))),
    };

    let small_threshold = 64 * 1024usize;
    let mut abort_rx_opt = abort_rx;
    let resp = match send_request(request, small_threshold, abort_rx_opt.take()).await {
        Ok(r) => r,
        Err(e) => return finalize_sink(sink_opt, Err(e)),
    };
    if !resp.status.is_success() {
        return finalize_sink(sink_opt, Err(format!("http status: {}", resp.status)));
    }

    let mut sink_active = true;
    let forward_to_sink =
        |data: &[u8], sink_opt: &mut Option<Box<dyn BodySink>>, active: &mut bool| {
            if *active {
                if let Some(ref mut sink) = sink_opt.as_mut() {
                    if let Err(err) = sink.write(data) {
                        let sink_err = Err(err.clone());
                        sink.close(&sink_err);
                        *sink_opt = None;
                        *active = false;
                    }
                }
            }
        };

    match resp.body {
        HttpBody::Empty => {}
        HttpBody::Small(buf) => {
            forward_to_sink(buf.as_ref(), &mut sink_opt, &mut sink_active);
            if let Err(e) = file.write_all(buf.as_ref()).await {
                return finalize_sink(sink_opt, Err(format!("write chunk: {}", e)));
            }
        }
        HttpBody::Stream(mut rx_body) => {
            while let Some(chunk_res) = rx_body.recv().await {
                let bytes = match chunk_res {
                    Ok(b) => b,
                    Err(e) => return finalize_sink(sink_opt, Err(e)),
                };

                forward_to_sink(bytes.as_ref(), &mut sink_opt, &mut sink_active);

                if let Err(e) = file.write_all(bytes.as_ref()).await {
                    return finalize_sink(sink_opt, Err(format!("write chunk: {}", e)));
                }
            }
        }
    }

    if let Err(e) = file.flush().await {
        return finalize_sink(sink_opt, Err(format!("flush: {}", e)));
    }
    drop(file);

    if let Err(e) = tokio::fs::rename(&temp_path, dest).await {
        return finalize_sink(sink_opt, Err(format!("rename: {}", e)));
    }

    finalize_sink(sink_opt, Ok(()))
}

fn finalize_sink(
    sink_opt: Option<Box<dyn BodySink>>,
    res: Result<(), String>,
) -> Result<(), String> {
    if let Some(mut sink) = sink_opt {
        sink.close(&res);
    }
    res
}

pub fn request_download(
    url: impl Into<String>,
    dest: impl Into<PathBuf>,
    abort_rx: Option<oneshot::Receiver<()>>,
    sink: Option<Box<dyn BodySink>>,
) -> Result<oneshot::Receiver<Result<(), String>>, String> {
    let (completion_tx, completion_rx) = oneshot::channel();

    let tx = {
        let guard = runtime_slot().lock().unwrap();
        if let Some(rt) = guard.as_ref() {
            rt.tx.clone()
        } else {
            let _ = completion_tx.send(Err("net runtime not started".to_string()));
            return Err("net runtime not started".to_string());
        }
    };

    let cmd = NetCommand::Download {
        url: url.into(),
        dest: dest.into(),
        abort_rx,
        sink,
        completion: completion_tx,
    };

    if let Err(e) = tx.blocking_send(cmd) {
        let err = format!("net service down: {}", e);
        if let NetCommand::Download { completion, .. } = e.0 {
            let _ = completion.send(Err(err.clone()));
        }
        return Err(err);
    }

    Ok(completion_rx)
}
