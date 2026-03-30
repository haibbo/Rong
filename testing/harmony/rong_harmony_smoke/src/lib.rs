use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use rong_test_harness::{LogLevel, run_tests};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::OnceLock;
use std::thread;
use tokio::net::TcpListener;
use tokio::runtime::Builder;

const LOG_TAG: &str = "RongSmoke";
static SERVER_START: OnceLock<Result<u16, String>> = OnceLock::new();

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct TestConfig {
    filter: String,
}

#[derive(Debug, Serialize)]
struct ServerStatus {
    ok: bool,
    status: &'static str,
    port: u16,
}

type Body = Full<Bytes>;

#[cfg(target_env = "ohos")]
fn init_logging_once() {
    use log::LevelFilter;
    use ohos_hilog::Config;

    ohos_hilog::init_once(
        Config::default()
            .with_max_level(LevelFilter::Info)
            .with_tag(LOG_TAG),
    );
}

#[cfg(not(target_env = "ohos"))]
fn init_logging_once() {}

fn log_fn(level: LogLevel, msg: &str) {
    match level {
        LogLevel::Info => {
            #[cfg(target_env = "ohos")]
            log::info!("{}", msg);
            #[cfg(not(target_env = "ohos"))]
            println!("[{}] {}", LOG_TAG, msg);
        }
        LogLevel::Error => {
            #[cfg(target_env = "ohos")]
            log::error!("{}", msg);
            #[cfg(not(target_env = "ohos"))]
            eprintln!("[{}] {}", LOG_TAG, msg);
        }
    }
}

fn json_response(status: StatusCode, body: String) -> Response<Body> {
    let mut response = Response::new(Full::new(Bytes::from(body)));
    *response.status_mut() = status;
    response.headers_mut().insert(
        hyper::header::CONTENT_TYPE,
        hyper::header::HeaderValue::from_static("application/json"),
    );
    response
}

fn run_configured_tests_impl(config_json: &str) -> Result<String, String> {
    init_logging_once();

    let config: TestConfig =
        serde_json::from_str(config_json).map_err(|e| format!("invalid test config: {e}"))?;
    let filter = config.filter.trim();
    let filter_display = if filter.is_empty() { "<all>" } else { filter };
    log_fn(
        LogLevel::Info,
        &format!("HTTP_RUN_START filter={filter_display}"),
    );

    let result = catch_unwind(AssertUnwindSafe(|| {
        let tests = rong_test_device::all_tests();
        let report = run_tests(&tests, filter, log_fn);
        serde_json::to_string(&report)
            .unwrap_or_else(|e| format!(r#"{{"ok":false,"error":"serialize: {e}"}}"#))
    }));

    match result {
        Ok(json) => {
            log_fn(LogLevel::Info, "HTTP_RUN_DONE");
            Ok(json)
        }
        Err(_) => {
            let message = "panic in Harmony test runner";
            log_fn(LogLevel::Error, message);
            Err(message.to_string())
        }
    }
}

async fn handle_request(
    request: Request<Incoming>,
    port: u16,
) -> Result<Response<Body>, Infallible> {
    let response = match (request.method(), request.uri().path()) {
        (&Method::GET, "/health") => {
            let body = serde_json::to_string(&ServerStatus {
                ok: true,
                status: "ready",
                port,
            })
            .unwrap_or_else(|_| r#"{"ok":false,"status":"serialize"}"#.to_string());
            json_response(StatusCode::OK, body)
        }
        (&Method::POST, "/run") => match request.into_body().collect().await {
            Ok(collected) => {
                let body_bytes = collected.to_bytes();
                let body = String::from_utf8_lossy(&body_bytes).into_owned();
                match thread::spawn(move || run_configured_tests_impl(&body)).join() {
                    Ok(Ok(json)) => json_response(StatusCode::OK, json),
                    Ok(Err(error)) => json_response(
                        StatusCode::BAD_REQUEST,
                        format!(r#"{{"ok":false,"error":"{error}"}}"#),
                    ),
                    Err(_) => json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        r#"{"ok":false,"error":"http worker panicked"}"#.to_string(),
                    ),
                }
            }
            Err(error) => json_response(
                StatusCode::BAD_REQUEST,
                format!(r#"{{"ok":false,"error":"read body: {error}"}}"#),
            ),
        },
        _ => json_response(
            StatusCode::NOT_FOUND,
            r#"{"ok":false,"error":"unknown endpoint"}"#.to_string(),
        ),
    };

    Ok(response)
}

fn start_http_server_impl(port: u16) -> Result<String, String> {
    init_logging_once();

    let result = SERVER_START.get_or_init(|| {
        thread::Builder::new()
            .name("rong-harmony-http".to_string())
            .spawn(move || {
                let runtime = match Builder::new_current_thread().enable_all().build() {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        log_fn(
                            LogLevel::Error,
                            &format!("build tokio runtime failed: {error}"),
                        );
                        return;
                    }
                };

                runtime.block_on(async move {
                    let listener = match TcpListener::bind(("0.0.0.0", port)).await {
                        Ok(listener) => listener,
                        Err(error) => {
                            log_fn(
                                LogLevel::Error,
                                &format!("bind port {port} failed: {error}"),
                            );
                            return;
                        }
                    };

                    log_fn(
                        LogLevel::Info,
                        &format!("HTTP server listening on 0.0.0.0:{port}"),
                    );

                    loop {
                        let (stream, _) = match listener.accept().await {
                            Ok(pair) => pair,
                            Err(error) => {
                                log_fn(LogLevel::Error, &format!("accept failed: {error}"));
                                continue;
                            }
                        };

                        let io = TokioIo::new(stream);
                        tokio::task::spawn(async move {
                            let service = service_fn(move |request| handle_request(request, port));
                            if let Err(error) =
                                http1::Builder::new().serve_connection(io, service).await
                            {
                                log_fn(
                                    LogLevel::Error,
                                    &format!("serve connection failed: {error}"),
                                );
                            }
                        });
                    }
                });
            })
            .map_err(|e| format!("spawn server thread: {e}"))?;

        Ok(port)
    });

    match result {
        Ok(port) => serde_json::to_string(&ServerStatus {
            ok: true,
            status: "ready",
            port: *port,
        })
        .map_err(|e| format!("serialize status: {e}")),
        Err(error) => Err(error.clone()),
    }
}

#[cfg_attr(
    target_env = "ohos",
    napi_derive_ohos::napi(js_name = "startHttpServer")
)]
pub fn start_http_server(port: u16) -> String {
    match start_http_server_impl(port) {
        Ok(json) => json,
        Err(error) => format!(r#"{{"ok":false,"error":"{error}"}}"#),
    }
}
