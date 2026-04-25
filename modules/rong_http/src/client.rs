use bytes::Bytes;
use http::Request as HttpRequest;
use http_body_util::combinators::BoxBody;
use std::io::Error;
use std::sync::Arc;
use tokio::sync::oneshot;

pub(crate) use rong_rt::http::{
    HttpBody, HttpError, HttpResponse, NetworkAccessGuard, RequestOptions,
};

struct FetchNetworkAccessGuard;

impl NetworkAccessGuard for FetchNetworkAccessGuard {
    fn check_access(&self, uri: &rong_rt::http::Uri) -> Result<(), HttpError> {
        let host = uri
            .host()
            .ok_or_else(|| HttpError::access_denied("Network access denied: URL has no host"))?;
        crate::security::grant_network_access(host)
            .map_err(|err| HttpError::access_denied(err.to_string()))
    }
}

pub(crate) async fn send_fetch_request(
    request: HttpRequest<BoxBody<Bytes, Error>>,
    small_threshold: usize,
    abort_rx: Option<oneshot::Receiver<()>>,
) -> Result<HttpResponse, HttpError> {
    rong_rt::http::send_with_small_body_limit(
        request,
        small_threshold,
        RequestOptions::new()
            .with_abort_opt(abort_rx)
            .with_network_access_guard(Arc::new(FetchNetworkAccessGuard)),
    )
    .await
}
