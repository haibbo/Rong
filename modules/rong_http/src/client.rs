use bytes::Bytes;
use http::Request as HttpRequest;
use http_body_util::combinators::BoxBody;
use std::io::Error;
use tokio::sync::oneshot;

pub use rong_rt::http::{
    BytesResponse, HttpBody, HttpError, HttpErrorKind, HttpResponse, JsonResponse, RequestOptions,
    clear_proxy, collect_body, default_timeout, post_json, post_json_bytes, proxy,
    reset_default_timeout, send, send_bytes, send_json, send_json_bytes, send_json_request,
    send_stream, set_default_timeout, set_proxy,
};

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
