use bytes::Bytes;
use http::Request as HttpRequest;
use http_body_util::combinators::BoxBody;
use std::io::Error;
use tokio::sync::oneshot;

pub(crate) use rong_rt::http::{HttpBody, HttpError, HttpResponse, RequestOptions};

pub(crate) async fn send_fetch_request(
    request: HttpRequest<BoxBody<Bytes, Error>>,
    small_threshold: usize,
    abort_rx: Option<oneshot::Receiver<()>>,
) -> Result<HttpResponse, HttpError> {
    rong_rt::http::send_with_small_body_limit(
        request,
        small_threshold,
        RequestOptions::new().with_abort_opt(abort_rx),
    )
    .await
}
