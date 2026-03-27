use bytes::Bytes;
use http::Request as HttpRequest;
use http::header;
use http_body_util::{BodyExt, Full};
use std::io::Error;
use tokio::io::AsyncWriteExt;
use tokio::sync::oneshot;

use crate::client::{HttpBody, send_request_with_timeout};
use tokio::time::Duration;

const DEFAULT_DOWNLOAD_SMALL_THRESHOLD: usize = 64 * 1024;

pub struct DownloadOptions {
    pub url: String,
    pub dest: std::path::PathBuf,
    pub sink: Option<Box<dyn BodySink>>,
    pub request_timeout: Option<Duration>,
}

impl DownloadOptions {
    /// Build download options for a target URL and destination file path.
    pub fn new(url: impl Into<String>, dest: impl Into<std::path::PathBuf>) -> Self {
        Self {
            url: url.into(),
            dest: dest.into(),
            sink: None,
            request_timeout: None,
        }
    }

    /// Mirror each downloaded chunk into an additional sink.
    pub fn with_sink(mut self, sink: Box<dyn BodySink>) -> Self {
        self.sink = Some(sink);
        self
    }

    /// Override the request timeout for this download.
    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = Some(timeout);
        self
    }
}

pub trait BodySink: Send {
    fn write(&mut self, chunk: &[u8]) -> Result<(), String>;
    fn close(&mut self, result: &Result<(), String>);
}

/// Download a resource directly on the current task.
pub async fn download(
    options: DownloadOptions,
    abort_rx: Option<oneshot::Receiver<()>>,
) -> Result<(), String> {
    download_resource(
        &options.url,
        &options.dest,
        abort_rx,
        options.sink,
        options.request_timeout,
    )
    .await
}

/// Spawn a background download and receive completion through a oneshot channel.
pub fn spawn_download(
    options: DownloadOptions,
    abort_rx: Option<oneshot::Receiver<()>>,
) -> Result<oneshot::Receiver<Result<(), String>>, String> {
    request_download_inner(
        options.url,
        options.dest,
        abort_rx,
        options.sink,
        options.request_timeout,
    )
}

pub fn request_download(
    url: impl Into<String>,
    dest: impl Into<std::path::PathBuf>,
    abort_rx: Option<oneshot::Receiver<()>>,
    sink: Option<Box<dyn BodySink>>,
) -> Result<oneshot::Receiver<Result<(), String>>, String> {
    request_download_inner(url, dest, abort_rx, sink, None)
}

pub fn request_download_with_timeout(
    url: impl Into<String>,
    dest: impl Into<std::path::PathBuf>,
    abort_rx: Option<oneshot::Receiver<()>>,
    sink: Option<Box<dyn BodySink>>,
    request_timeout: Duration,
) -> Result<oneshot::Receiver<Result<(), String>>, String> {
    request_download_inner(url, dest, abort_rx, sink, Some(request_timeout))
}

fn request_download_inner(
    url: impl Into<String>,
    dest: impl Into<std::path::PathBuf>,
    abort_rx: Option<oneshot::Receiver<()>>,
    sink: Option<Box<dyn BodySink>>,
    timeout_override: Option<Duration>,
) -> Result<oneshot::Receiver<Result<(), String>>, String> {
    let (completion_tx, completion_rx) = oneshot::channel();

    let url = url.into();
    let dest = dest.into();
    crate::RongExecutor::global().spawn(async move {
        let res = download_resource(&url, &dest, abort_rx, sink, timeout_override).await;
        let _ = completion_tx.send(res);
    });

    Ok(completion_rx)
}

async fn download_resource(
    url: &str,
    dest: &std::path::PathBuf,
    abort_rx: Option<oneshot::Receiver<()>>,
    sink: Option<Box<dyn BodySink>>,
    timeout_override: Option<Duration>,
) -> Result<(), String> {
    let mut sink_opt = sink;

    if let Some(parent) = dest.parent()
        && let Err(e) = tokio::fs::create_dir_all(parent).await
    {
        return finalize_sink(sink_opt, Err(format!("create dir: {}", e)));
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
        let ua = crate::get_user_agent();
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
        .map_err(|_| Error::other("body error"))
        .boxed();
    let request = match builder.body(empty) {
        Ok(req) => req,
        Err(e) => {
            return finalize_sink(sink_opt, Err(format!("build request: {}", e)));
        }
    };

    let small_threshold = DEFAULT_DOWNLOAD_SMALL_THRESHOLD;
    let mut abort_rx_opt = abort_rx;
    let resp = match send_request_with_timeout(
        request,
        small_threshold,
        abort_rx_opt.take(),
        timeout_override,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => return finalize_sink(sink_opt, Err(e)),
    };
    if !resp.status.is_success() {
        return finalize_sink(sink_opt, Err(format!("http status: {}", resp.status)));
    }

    let mut sink_active = true;
    let forward = |data: &[u8], sink_opt: &mut Option<Box<dyn BodySink>>, active: &mut bool| {
        if *active
            && let Some(sink) = sink_opt.as_mut()
            && let Err(err) = sink.write(data)
        {
            let sink_err = Err(err.clone());
            sink.close(&sink_err);
            *sink_opt = None;
            *active = false;
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

#[cfg(test)]
mod tests {
    use super::*;

    // Pool starts lazily on first spawn/handle; nothing to do here.

    async fn spawn_file_server(content: &'static [u8]) -> std::net::SocketAddr {
        use axum::Router;
        use axum::routing::get;

        let app = Router::new().route("/file", get(move || async move { content }));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        addr
    }

    async fn spawn_slow_server(delay_ms: u64) -> std::net::SocketAddr {
        use axum::Router;
        use axum::routing::get;

        let app = Router::new().route(
            "/slow",
            get(move || async move {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                "data"
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
    fn spawn_download_with_options_succeeds() {
        let handle = crate::RongExecutor::global().handle();
        handle.block_on(async {
            let content = b"hello download";
            let addr = spawn_file_server(content).await;
            let url = format!("http://{}/file", addr);

            let dest = std::env::temp_dir().join(format!(
                "rong_dl_test_{}.bin",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos()
            ));

            let rx = spawn_download(
                DownloadOptions::new(&url, &dest).with_request_timeout(Duration::from_secs(5)),
                None,
            )
            .expect("should queue download");
            rx.await
                .expect("channel dropped")
                .expect("download should succeed");

            let written = tokio::fs::read(&dest).await.expect("file should exist");
            assert_eq!(written, content);
            let _ = tokio::fs::remove_file(&dest).await;
        });
    }

    #[test]
    fn download_convenience_succeeds() {
        let handle = crate::RongExecutor::global().handle();
        handle.block_on(async {
            let content = b"hello direct download";
            let addr = spawn_file_server(content).await;
            let url = format!("http://{}/file", addr);
            let dest = std::env::temp_dir().join(format!(
                "rong_dl_direct_test_{}.bin",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos()
            ));

            download(DownloadOptions::new(&url, &dest), None)
                .await
                .expect("download should succeed");

            let written = tokio::fs::read(&dest).await.expect("file should exist");
            assert_eq!(written, content);
            let _ = tokio::fs::remove_file(&dest).await;
        });
    }

    #[test]
    fn download_with_timeout_expires() {
        let handle = crate::RongExecutor::global().handle();
        handle.block_on(async {
            let addr = spawn_slow_server(300).await;
            let url = format!("http://{}/slow", addr);
            let dest = std::env::temp_dir().join("rong_dl_timeout_test.bin");

            let rx =
                request_download_with_timeout(&url, &dest, None, None, Duration::from_millis(10))
                    .expect("should queue download");
            let err = rx
                .await
                .expect("channel dropped")
                .expect_err("should time out");
            assert!(
                err.contains("timeout"),
                "expected timeout error, got: {}",
                err
            );
        });
    }
}
