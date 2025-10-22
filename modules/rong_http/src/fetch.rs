use http::Request as HttpRequest;
use http::header;
use http_body::Frame;
use http_body_util::{BodyExt, Full, StreamBody, combinators::BoxBody};
use hyper::body::Bytes;
use rong::{function::Optional, *};
use std::io::Error;
use tokio::sync::oneshot;
use tokio_stream::{StreamExt as _, wrappers::ReceiverStream};

use crate::formdata::FormData;
use crate::request::{Request, RequestInit};
use crate::response::Response;
use crate::security::grant_network_access;
use rong::net;
use rong_stream::ReadableStream;

// Convert Request to hyper::Request
async fn to_hyper_request(mut request: Request) -> JSResult<HttpRequest<BoxBody<Bytes, Error>>> {
    let mut builder = HttpRequest::builder()
        .method(request.method)
        .uri(request.url)
        .header(header::USER_AGENT, rong_navigator::get_user_agent())
        .header(header::ACCEPT, "*/*")
        .header(header::ACCEPT_ENCODING, "gzip"); // TODO: "gzip, zstd"

    // Take ownership of headers
    if let Some(headers) = builder.headers_mut() {
        headers.extend(request.headers.into_header_map());
    }

    // Create body
    let body: BoxBody<Bytes, Error> = if let Some(body) = request.body.take() {
        // Detect ReadableStream for streaming upload
        if let Some(obj) = body.0.clone().into_object() {
            if obj.borrow::<ReadableStream>().is_ok() {
                if let Ok(rs) = obj.borrow::<ReadableStream>() {
                    if let Some(rx) = rong_stream::readable_stream_take_receiver(&rs) {
                        let stream = ReceiverStream::new(rx).map(|item| match item {
                            Ok(bytes) => Ok(Frame::data(bytes)),
                            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
                        });
                        let sb = StreamBody::new(stream);
                        sb.boxed()
                    } else {
                        Full::new(Bytes::new()).map_err(|e| match e {}).boxed()
                    }
                } else {
                    Full::new(Bytes::new()).map_err(|e| match e {}).boxed()
                }
            } else {
                // Fallback to non-streaming conversion
                let (bytes, boundary) = body.to_bytes().await.unwrap_or_default();
                if let Some(boundary) = boundary {
                    if let Some(headers) = builder.headers_mut() {
                        headers.insert(
                            header::CONTENT_TYPE,
                            FormData::content_type(&boundary).parse().map_err(|e| {
                                RongJSError::TypeError(format!(
                                    "Invalid content-type header: {}",
                                    e
                                ))
                            })?,
                        );
                    }
                }
                Full::new(bytes).map_err(|e| match e {}).boxed()
            }
        } else {
            let (bytes, boundary) = body.to_bytes().await.unwrap_or_default();
            if let Some(boundary) = boundary {
                if let Some(headers) = builder.headers_mut() {
                    headers.insert(
                        header::CONTENT_TYPE,
                        FormData::content_type(&boundary).parse().map_err(|e| {
                            RongJSError::TypeError(format!("Invalid content-type header: {}", e))
                        })?,
                    );
                }
            }
            Full::new(bytes).map_err(|e| match e {}).boxed()
        }
    } else {
        Full::new(Bytes::new()).map_err(|e| match e {}).boxed()
    };

    // builder.body() only fails if the headers are invalid, which we know they aren't
    // because we just created them from valid headers
    builder
        .body(body)
        .map_err(|e| RongJSError::TypeError(format!("Failed to build request: {}", e)))
}

pub async fn fetch(input: JSValue, init: Optional<RequestInit>) -> JSResult<Response> {
    // Create Request object from input and init
    let request = Request::new(input, init).map_err(|e| RongJSError::TypeError(e.to_string()))?;

    // Check network access permission
    let domain = request.domain()?;
    grant_network_access(&domain)?;

    // Get abort signal if present
    let abort_receiver = request.abort_signal().map(|signal| signal.subscribe());

    // Convert Request to hyper::Request
    let hyper_request = to_hyper_request(request).await?;
    let orig_method = hyper_request.method().clone();
    let orig_uri = hyper_request.uri().clone();

    // Send via dedicated net service
    let abort_bridge = if let Some(mut r) = abort_receiver.clone() {
        let (tx, rx) = oneshot::channel::<()>();
        tokio::task::spawn_local(async move {
            let _ = r.recv().await;
            let _ = tx.send(());
        });
        Some(rx)
    } else {
        None
    };

    let small_threshold = 4096usize;
    // Race the network request with an early abort. If the abort wins, reject with its reason.
    let net_fut = net::send_request(hyper_request, small_threshold, abort_bridge);
    let net_resp = if let Some(mut early_abort) = abort_receiver.clone() {
        tokio::select! {
            res = net_fut => res.map_err(|e| RongJSError::TypeError(format!("fetch failed: {}", e)))?,
            reason = early_abort.recv() => {
                return Err(RongJSError::from_jsvalue(reason));
            }
        }
    } else {
        net_fut
            .await
            .map_err(|e| RongJSError::TypeError(format!("fetch failed: {}", e)))?
    };

    let body_kind = if let Some(buf) = net_resp.small_buffer {
        crate::body::BodyKind::Buffered(buf)
    } else if let Some(rx) = net_resp.body_rx {
        crate::body::BodyKind::Channel(std::sync::Arc::new(std::sync::Mutex::new(Some(rx))))
    } else {
        crate::body::BodyKind::Buffered(Vec::new())
    };

    Ok(Response::from_meta(
        net_resp.status,
        net_resp.headers,
        body_kind,
        abort_receiver,
        orig_method,
        orig_uri,
    ))
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    let fetch_fn = JSFunc::new(ctx, fetch)?;
    ctx.global().set("fetch", fetch_fn)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        http::HeaderMap,
        response::{IntoResponse, Response as AxumResponse},
        routing::get,
    };
    use flate2::{Compression, write::GzEncoder};
    use futures::{StreamExt, stream};
    use rong_test::*;
    use std::convert::Infallible;
    use std::io::Write;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;
    use tokio::time::{Duration, sleep};

    async fn test_ip() -> impl IntoResponse {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());

        AxumResponse::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"origin": "127.0.0.1"}"#))
            .unwrap()
    }

    async fn test_gzip() -> impl IntoResponse {
        // Create gzipped JSON response
        let json = r#"{"gzipped": true, "method": "GET"}"#;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(json.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();

        AxumResponse::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::CONTENT_ENCODING, "gzip")
            .body(Body::from(compressed))
            .unwrap()
    }

    async fn test_delay() -> impl IntoResponse {
        sleep(Duration::from_millis(1000)).await;
        AxumResponse::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"delayed": true}"#))
            .unwrap()
    }

    async fn test_large() -> impl IntoResponse {
        let stream = stream::iter(0..100).then(|i| async move {
            // Add a significant delay between chunks
            sleep(Duration::from_millis(200)).await;
            Ok::<_, Infallible>(format!("chunk_{:04}\n", i).repeat(1024))
        });

        // Convert the stream into a response
        AxumResponse::builder()
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from_stream(stream))
            .unwrap()
    }

    async fn test_headers(headers: HeaderMap) -> impl IntoResponse {
        // Create a response containing the received headers
        let mut headers_map = serde_json::Map::new();
        for (key, value) in headers.iter() {
            if let Ok(v) = value.to_str() {
                headers_map.insert(
                    key.as_str().to_string(),
                    serde_json::Value::String(v.to_string()),
                );
            }
        }
        let json = serde_json::Value::Object(headers_map);

        AxumResponse::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(json.to_string()))
            .unwrap()
    }

    async fn start_test_server() -> SocketAddr {
        let app = Router::new()
            .route("/ip", get(test_ip))
            .route("/gzip", get(test_gzip))
            .route("/delay", get(test_delay))
            .route("/large", get(test_large))
            .route("/headers", get(test_headers));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        addr
    }

    #[test]
    fn test_fetch() {
        async_run!(|ctx: JSContext| async move {
            rong_assert::init(&ctx)?;
            rong_console::init(&ctx)?;
            rong_encoding::init(&ctx)?;
            rong_url::init(&ctx)?;
            rong_timer::init(&ctx)?;
            rong_abort::init(&ctx)?;
            rong_exception::init(&ctx)?;

            crate::header::init(&ctx)?;
            crate::request::init(&ctx)?;
            crate::response::init(&ctx)?;
            init(&ctx)?;

            // Start test server
            let addr = start_test_server().await;
            let base_url = format!("http://{}", addr);

            // Set base URL for tests
            ctx.global().set("TEST_SERVER_URL", base_url)?;

            let passed = UnitJSRunner::load_script(&ctx, "fetch.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }

    #[test]
    fn test_network_access_guard() {
        use crate::security::{NetworkAccessGuard, grant_network_access, set_network_access_guard};
        use rong::RongJSError;

        // Test default guard allows all domains
        let result = grant_network_access("httpbin.org");
        assert!(result.is_ok());

        // Define restricted network guard
        struct RestrictedNetworkGuard;

        impl NetworkAccessGuard for RestrictedNetworkGuard {
            fn check_access(&self, domain: &str) -> JSResult<()> {
                if domain == "allowed.example.com" {
                    Ok(())
                } else {
                    Err(RongJSError::TypeError("Network access denied".to_string()))
                }
            }
        }

        // Set restricted network guard
        set_network_access_guard(Box::new(RestrictedNetworkGuard));

        // Test allowed domain - should succeed
        let allowed_result = grant_network_access("allowed.example.com");
        assert!(allowed_result.is_ok());

        // Test denied domain - should fail
        let denied_result = grant_network_access("denied.example.com");
        assert!(denied_result.is_err());
        if let Err(err) = denied_result {
            assert!(err.to_string().contains("Network access denied"));
        }
    }
}
