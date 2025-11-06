use super::{ServiceCommand, get_user_agent, net::HttpBody, send_request};
use bytes::Bytes;
use http::Request as HttpRequest;
use http::header;
use http_body_util::{BodyExt, Full};
use std::io::{Error, ErrorKind};
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc::error::TrySendError, oneshot};

const DEFAULT_DOWNLOAD_SMALL_THRESHOLD: usize = 64 * 1024;

pub trait BodySink: Send {
    fn write(&mut self, chunk: &[u8]) -> Result<(), String>;
    fn close(&mut self, result: &Result<(), String>);
}

pub fn request_download(
    url: impl Into<String>,
    dest: impl Into<std::path::PathBuf>,
    abort_rx: Option<oneshot::Receiver<()>>,
    sink: Option<Box<dyn BodySink>>,
) -> Result<oneshot::Receiver<Result<(), String>>, String> {
    let (completion_tx, completion_rx) = oneshot::channel();

    let tx = {
        let guard = super::runtime::runtime_slot().lock().unwrap();
        if let Some(rt) = guard.as_ref() {
            rt.tx.clone()
        } else {
            let _ = completion_tx.send(Err("service runtime not started".to_string()));
            return Err("service runtime not started".to_string());
        }
    };

    let cmd = ServiceCommand::Download {
        url: url.into(),
        dest: dest.into(),
        abort_rx,
        sink,
        completion: completion_tx,
    };

    match tx.try_send(cmd) {
        Ok(_) => Ok(completion_rx),
        Err(TrySendError::Full(cmd)) => {
            if let ServiceCommand::Download { completion, .. } = cmd {
                let _ = completion.send(Err("service runtime queue full".to_string()));
            }
            Err("service runtime queue full".to_string())
        }
        Err(TrySendError::Closed(cmd)) => {
            if let ServiceCommand::Download { completion, .. } = cmd {
                let _ = completion.send(Err("service runtime down".to_string()));
            }
            Err("service runtime down".to_string())
        }
    }
}

pub(super) async fn download_resource(
    url: &str,
    dest: &std::path::PathBuf,
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
        match header::HeaderValue::from_str(&ua) {
            Ok(v) => {
                headers.insert(header::USER_AGENT, v);
            }
            Err(e) => {
                return finalize_sink(sink_opt, Err(format!("invalid user agent header: {}", e)));
            }
        }
    }
    let empty = Full::new(Bytes::new())
        .map_err(|_| Error::new(ErrorKind::Other, "body error"))
        .boxed();
    let request = match builder.body(empty) {
        Ok(req) => req,
        Err(e) => {
            return finalize_sink(sink_opt, Err(format!("build request: {}", e)));
        }
    };

    let small_threshold = DEFAULT_DOWNLOAD_SMALL_THRESHOLD;
    let mut abort_rx_opt = abort_rx;
    let resp = match send_request(request, small_threshold, abort_rx_opt.take()).await {
        Ok(r) => r,
        Err(e) => return finalize_sink(sink_opt, Err(e)),
    };
    if !resp.status.is_success() {
        return finalize_sink(sink_opt, Err(format!("http status: {}", resp.status)));
    }

    let mut sink_active = true;
    let forward = |data: &[u8], sink_opt: &mut Option<Box<dyn BodySink>>, active: &mut bool| {
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
            forward(buf.as_ref(), &mut sink_opt, &mut sink_active);
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

                forward(bytes.as_ref(), &mut sink_opt, &mut sink_active);

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
