use http::header;
use http::Request as HttpRequest;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::body::Bytes;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use rusty_js::{function::Optional, *};
use std::io::Error;
use std::sync::OnceLock;

use crate::request::{Request, RequestInit};
use crate::response::Response;

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
        let https = HttpsConnectorBuilder::new()
            .with_webpki_roots()
            .https_or_http()
            .enable_http1()
            .build();

        Client::builder(TokioExecutor::new()).build(https)
    })
}

// Convert Request to hyper::Request
fn to_hyper_request(request: Request) -> HttpRequest<BoxBody<Bytes, Error>> {
    let mut builder = HttpRequest::builder()
        .method(request.method)
        .uri(request.url)
        .header(header::ACCEPT_ENCODING, "gzip"); // TODO: "gzip, zstd"

    // Take ownership of headers
    if let Some(headers) = builder.headers_mut() {
        headers.extend(request.headers.into_header_map());
    }

    // Create body - if conversion fails, use empty body
    let bytes = request
        .body
        .and_then(|b| b.to_bytes().ok())
        .unwrap_or_default();

    let body = Full::new(bytes)
        .map_err(|e| match e {}) // Infallible can never happen
        .boxed();

    // builder.body() only fails if the headers are invalid, which we know they aren't
    // because we just created them from valid headers
    builder.body(body).unwrap()
}

pub async fn fetch(input: JSValue, init: Optional<RequestInit>) -> JSResult<Response> {
    // Create Request object from input and init
    let request = Request::new(input, init)?;

    // Convert Request to hyper::Request
    let hyper_request = to_hyper_request(request);

    // Send request and get response
    let hyper_response = get_client()
        .request(hyper_request)
        .await
        .map_err(|e| RustyJSError::TypeError(format!("fetch failed: {}", e)))?;

    // Convert hyper::Response to our Response
    Ok(Response::from_hyper(hyper_response))
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.global().set("fetch", JSFunc::new(ctx, fetch))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::HeaderMap,
        response::{IntoResponse, Response as AxumResponse},
        routing::get,
        Router,
    };
    use flate2::{write::GzEncoder, Compression};
    use rustyjs_test::*;
    use std::io::Write;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

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

    async fn start_test_server() -> SocketAddr {
        let app = Router::new()
            .route("/ip", get(test_ip))
            .route("/gzip", get(test_gzip));

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
            assert::init(&ctx)?;
            console::init(&ctx, None)?;
            encoding::init(&ctx)?;
            lxr_url::init(&ctx)?;

            crate::blob::init(&ctx).unwrap();
            crate::header::init(&ctx).unwrap();
            crate::request::init(&ctx).unwrap();
            crate::response::init(&ctx).unwrap();
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
}
