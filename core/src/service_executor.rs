mod download;
mod net;
mod runtime;

pub use download::{BodySink, request_download};
pub use net::{HttpBody, HttpResponse, post_json, send_request};
pub use runtime::{
    get_user_agent, set_user_agent, spawn_async, spawn_blocking, start_service_runtime,
    stop_service_runtime,
};

use bytes::Bytes;
use http::Request as HttpRequest;
use http_body_util::combinators::BoxBody;
use std::io::Error;
use std::path::PathBuf;
use tokio::sync::oneshot;

pub(crate) struct HttpJob {
    pub request: HttpRequest<BoxBody<Bytes, Error>>,
    pub small_threshold: usize,
    pub resp_tx: oneshot::Sender<Result<net::HttpResponse, String>>,
    pub abort_rx: Option<oneshot::Receiver<()>>,
}

pub(crate) enum ServiceCommand {
    Http(HttpJob),
    Download {
        url: String,
        dest: PathBuf,
        abort_rx: Option<oneshot::Receiver<()>>,
        sink: Option<Box<dyn download::BodySink>>,
        completion: oneshot::Sender<Result<(), String>>,
    },
    Shutdown(oneshot::Sender<()>),
}
