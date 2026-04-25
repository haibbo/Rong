use bytes::Bytes;
use http::Request as HttpRequest;
use http::header;
use http_body_util::{BodyExt, Full};
use std::io::Error;
use tokio::io::AsyncWriteExt;
use tokio::sync::oneshot;

use crate::client::{
    HttpBody, RequestTimeouts, send_request_with_shared_abort, shared_abort_signal,
};
use tokio::time::{Duration, Instant};

const DEFAULT_DOWNLOAD_SMALL_THRESHOLD: usize = 64 * 1024;
const MAX_DOWNLOAD_REDIRECTS: usize = 10;

pub struct DownloadOptions {
    url: String,
    dest: std::path::PathBuf,
    sink: Option<Box<dyn BodySink>>,
    request_timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
}

impl DownloadOptions {
    /// Build download options for a target URL and destination file path.
    pub fn new(url: impl Into<String>, dest: impl Into<std::path::PathBuf>) -> Self {
        Self {
            url: url.into(),
            dest: dest.into(),
            sink: None,
            request_timeout: None,
            connect_timeout: None,
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

    /// Override the socket-connect timeout for this download.
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    fn timeouts(&self) -> RequestTimeouts {
        RequestTimeouts {
            request_timeout: self.request_timeout,
            connect_timeout: self.connect_timeout,
        }
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
    let timeouts = options.timeouts();
    download_resource(
        &options.url,
        &options.dest,
        abort_rx,
        options.sink,
        timeouts,
    )
    .await
}

/// Spawn a background download and receive completion through a oneshot channel.
pub fn spawn_download(
    options: DownloadOptions,
    abort_rx: Option<oneshot::Receiver<()>>,
) -> Result<oneshot::Receiver<Result<(), String>>, String> {
    let timeouts = options.timeouts();
    request_download_inner(options.url, options.dest, abort_rx, options.sink, timeouts)
}

pub fn request_download(
    url: impl Into<String>,
    dest: impl Into<std::path::PathBuf>,
    abort_rx: Option<oneshot::Receiver<()>>,
    sink: Option<Box<dyn BodySink>>,
) -> Result<oneshot::Receiver<Result<(), String>>, String> {
    request_download_inner(url, dest, abort_rx, sink, RequestTimeouts::default())
}

fn request_download_inner(
    url: impl Into<String>,
    dest: impl Into<std::path::PathBuf>,
    abort_rx: Option<oneshot::Receiver<()>>,
    sink: Option<Box<dyn BodySink>>,
    timeouts: RequestTimeouts,
) -> Result<oneshot::Receiver<Result<(), String>>, String> {
    let (completion_tx, completion_rx) = oneshot::channel();

    let url = url.into();
    let dest = dest.into();
    crate::RongExecutor::global().spawn(async move {
        let res = download_resource(&url, &dest, abort_rx, sink, timeouts).await;
        let _ = completion_tx.send(res);
    });

    Ok(completion_rx)
}

async fn download_resource(
    url: &str,
    dest: &std::path::PathBuf,
    abort_rx: Option<oneshot::Receiver<()>>,
    sink: Option<Box<dyn BodySink>>,
    timeouts: RequestTimeouts,
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

    let small_threshold = DEFAULT_DOWNLOAD_SMALL_THRESHOLD;
    let mut current_url = url.to_string();
    let mut redirect_count = 0usize;
    let abort_signal = shared_abort_signal(abort_rx);
    let request_deadline = timeouts
        .request_timeout
        .map(|timeout| Instant::now() + timeout);
    let resp = loop {
        let request = match build_download_request(&current_url) {
            Ok(request) => request,
            Err(e) => return finalize_sink(sink_opt, Err(e)),
        };
        let remaining_timeouts = match remaining_request_timeouts(timeouts, request_deadline) {
            Ok(timeouts) => timeouts,
            Err(e) => return finalize_sink(sink_opt, Err(e)),
        };
        let resp = match send_request_with_shared_abort(
            request,
            small_threshold,
            abort_signal.clone(),
            crate::client::DEFAULT_STREAM_COALESCE_TARGET,
            remaining_timeouts,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => return finalize_sink(sink_opt, Err(e)),
        };

        if !is_download_redirect(resp.status) {
            break resp;
        }
        if redirect_count >= MAX_DOWNLOAD_REDIRECTS {
            return finalize_sink(sink_opt, Err("too many redirects".to_string()));
        }
        current_url = match redirect_target(&current_url, &resp.headers) {
            Ok(url) => url,
            Err(e) => return finalize_sink(sink_opt, Err(e)),
        };
        redirect_count += 1;
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

fn remaining_request_timeouts(
    timeouts: RequestTimeouts,
    deadline: Option<Instant>,
) -> Result<RequestTimeouts, String> {
    let request_timeout = match deadline {
        Some(deadline) => {
            let now = Instant::now();
            if now >= deadline {
                return Err("request timeout".to_string());
            }
            Some(deadline - now)
        }
        None => timeouts.request_timeout,
    };

    Ok(RequestTimeouts {
        request_timeout,
        connect_timeout: timeouts.connect_timeout,
    })
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

fn build_download_request(
    url: &str,
) -> Result<HttpRequest<http_body_util::combinators::BoxBody<Bytes, Error>>, String> {
    let mut builder = HttpRequest::builder()
        .method("GET")
        .uri(url)
        .header(header::ACCEPT, "*/*");
    if let Some(headers) = builder.headers_mut() {
        let ua = crate::get_user_agent();
        let value = header::HeaderValue::from_str(&ua)
            .map_err(|e| format!("invalid user agent header: {}", e))?;
        headers.insert(header::USER_AGENT, value);
    }
    let empty = Full::new(Bytes::new())
        .map_err(|_| Error::other("body error"))
        .boxed();
    builder
        .body(empty)
        .map_err(|e| format!("build request: {}", e))
}

fn is_download_redirect(status: http::StatusCode) -> bool {
    matches!(status.as_u16(), 301 | 302 | 303 | 307 | 308)
}

fn redirect_target(current_url: &str, headers: &http::HeaderMap) -> Result<String, String> {
    let location = headers
        .get(header::LOCATION)
        .ok_or_else(|| "redirect response missing Location header".to_string())?
        .to_str()
        .map_err(|e| format!("invalid redirect Location header: {}", e))?
        .trim();
    if location.is_empty() {
        return Err("redirect Location header is empty".to_string());
    }

    let base = url::Url::parse(current_url)
        .map_err(|e| format!("invalid current URL for redirect: {}", e))?;
    let next = base
        .join(location)
        .map_err(|e| format!("invalid redirect URL: {}", e))?;
    match next.scheme() {
        "http" | "https" => Ok(next.to_string()),
        scheme => Err(format!("unsupported redirect URL scheme: {}", scheme)),
    }
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
        let _guard = crate::client::test_guard();
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
                DownloadOptions::new(&url, &dest)
                    .with_request_timeout(Duration::from_secs(5))
                    .with_connect_timeout(Duration::from_secs(1)),
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
        let _guard = crate::client::test_guard();
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
    fn redirect_target_resolves_relative_location() {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            header::LOCATION,
            header::HeaderValue::from_static("/next.jpg"),
        );
        let target = redirect_target("https://example.com/a/start.jpg", &headers).unwrap();
        assert_eq!(target, "https://example.com/next.jpg");
    }

    #[test]
    fn redirect_target_rejects_non_http_scheme() {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            header::LOCATION,
            header::HeaderValue::from_static("file:///tmp/a.jpg"),
        );
        let err = redirect_target("https://example.com/a/start.jpg", &headers).unwrap_err();
        assert!(err.contains("unsupported redirect URL scheme"));
    }

    #[test]
    fn download_follows_redirect() {
        let _guard = crate::client::test_guard();
        let handle = crate::RongExecutor::global().handle();
        handle.block_on(async {
            use axum::Router;
            use axum::response::IntoResponse;
            use axum::routing::get;

            let content = b"redirected download";
            let app = Router::new()
                .route("/file", get(move || async move { content }))
                .route(
                    "/redirect",
                    get(|| async {
                        (
                            http::StatusCode::FOUND,
                            [(http::header::LOCATION, "/file")],
                            "",
                        )
                            .into_response()
                    }),
                );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });

            let url = format!("http://{}/redirect", addr);
            let dest = std::env::temp_dir().join(format!(
                "rong_dl_redirect_test_{}.bin",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos()
            ));

            download(DownloadOptions::new(&url, &dest), None)
                .await
                .expect("download should follow redirect");

            let written = tokio::fs::read(&dest).await.expect("file should exist");
            assert_eq!(written, content);
            let _ = tokio::fs::remove_file(&dest).await;
        });
    }

    #[test]
    fn download_abort_survives_redirect() {
        let _guard = crate::client::test_guard();
        let handle = crate::RongExecutor::global().handle();
        handle.block_on(async {
            use axum::Router;
            use axum::response::IntoResponse;
            use axum::routing::get;

            let app = Router::new()
                .route(
                    "/slow-file",
                    get(|| async {
                        tokio::time::sleep(Duration::from_millis(300)).await;
                        "redirected download"
                    }),
                )
                .route(
                    "/redirect",
                    get(|| async {
                        (
                            http::StatusCode::FOUND,
                            [(http::header::LOCATION, "/slow-file")],
                            "",
                        )
                            .into_response()
                    }),
                );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });

            let url = format!("http://{}/redirect", addr);
            let dest = std::env::temp_dir().join(format!(
                "rong_dl_redirect_abort_test_{}.bin",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos()
            ));
            let (abort_tx, abort_rx) = oneshot::channel();

            let rx = spawn_download(DownloadOptions::new(&url, &dest), Some(abort_rx))
                .expect("should queue download");
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _ = abort_tx.send(());

            let err = rx
                .await
                .expect("channel dropped")
                .expect_err("download should abort");
            assert!(
                err.contains("aborted"),
                "expected abort error, got: {}",
                err
            );
            let _ = tokio::fs::remove_file(&dest).await;
        });
    }

    #[test]
    fn download_redirects_share_request_timeout_budget() {
        let _guard = crate::client::test_guard();
        let handle = crate::RongExecutor::global().handle();
        handle.block_on(async {
            use axum::Router;
            use axum::response::IntoResponse;
            use axum::routing::get;

            let app = Router::new()
                .route(
                    "/slow-file",
                    get(|| async {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        "redirected download"
                    }),
                )
                .route(
                    "/redirect",
                    get(|| async {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        (
                            http::StatusCode::FOUND,
                            [(http::header::LOCATION, "/slow-file")],
                            "",
                        )
                            .into_response()
                    }),
                );
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });

            let url = format!("http://{}/redirect", addr);
            let dest = std::env::temp_dir().join(format!(
                "rong_dl_redirect_timeout_test_{}.bin",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos()
            ));

            let err = download(
                DownloadOptions::new(&url, &dest)
                    .with_request_timeout(Duration::from_millis(150))
                    .with_connect_timeout(Duration::from_secs(1)),
                None,
            )
            .await
            .expect_err("redirect chain should exhaust the timeout budget");

            assert!(
                err.contains("timeout"),
                "expected timeout error, got: {}",
                err
            );
            let _ = tokio::fs::remove_file(&dest).await;
        });
    }

    #[test]
    fn download_with_timeout_expires() {
        let _guard = crate::client::test_guard();
        let handle = crate::RongExecutor::global().handle();
        handle.block_on(async {
            let addr = spawn_slow_server(300).await;
            let url = format!("http://{}/slow", addr);
            let dest = std::env::temp_dir().join("rong_dl_timeout_test.bin");

            let rx = spawn_download(
                DownloadOptions::new(&url, &dest)
                    .with_request_timeout(Duration::from_millis(10))
                    .with_connect_timeout(Duration::from_secs(1)),
                None,
            )
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
