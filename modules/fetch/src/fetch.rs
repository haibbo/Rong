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
        .uri(request.url);

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
    use rustyjs_test::*;

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

            let passed = UnitJSRunner::load_script(&ctx, "fetch.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }
}
