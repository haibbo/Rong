use http::Request as HttpRequest;
use http::header;
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::body::Bytes;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use rong::{function::Optional, *};
use std::io::Error;
use std::sync::OnceLock;
use tokio::select;

use crate::formdata::FormData;
use crate::request::{Request, RequestInit};
use crate::response::Response;
use crate::security::grant_network_access;

// Global client instance
static CLIENT: OnceLock<
    Client<
        hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
        BoxBody<Bytes, Error>,
    >,
> = OnceLock::new();

// Create a new HTTPS-enabled client
fn get_client() -> &'static Client<
    hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
    BoxBody<Bytes, Error>,
> {
    CLIENT.get_or_init(|| {
        // Initialize rustls CryptoProvider
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        let https = HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1()
            .build();

        Client::builder(TokioExecutor::new()).build(https)
    })
}

// Convert Request to hyper::Request
async fn to_hyper_request(request: Request) -> JSResult<HttpRequest<BoxBody<Bytes, Error>>> {
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

    // Create body - if conversion fails, use empty body
    let (bytes, boundary) = if let Some(body) = request.body {
        body.to_bytes().await.ok()
    } else {
        None
    }
    .unwrap_or_default();

    // If we have a boundary, set the Content-Type header with it
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

    let body = Full::new(bytes)
        .map_err(|e| match e {}) // Infallible can never happen
        .boxed();

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
    let mut abort_receiver = request.abort_signal().map(|signal| signal.subscribe());

    // Convert Request to hyper::Request
    let hyper_request = to_hyper_request(request).await?;

    // Send request and get response
    let response = if let Some(ref mut receiver) = abort_receiver {
        select! {
            result = get_client().request(hyper_request) => {
                result.map_err(|e| RongJSError::TypeError(format!("fetch failed: {}", e)))?
            }
            abort_reason = receiver.recv() => {
                return Err(RongJSError::from_jsvalue(abort_reason));
            }
        }
    } else {
        get_client()
            .request(hyper_request)
            .await
            .map_err(|e| RongJSError::TypeError(format!("fetch failed: {}", e)))?
    };

    // Convert hyper::Response to our Response, passing the abort receiver
    Ok(Response::from_hyper(response, abort_receiver))
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
