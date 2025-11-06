use super::{ServiceCommand, download, net};
use bytes::Bytes;
use futures::executor::block_on;
use http::HeaderValue;
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use std::future::Future;
use std::sync::{Mutex, OnceLock};
use std::thread;
use tokio::runtime::{Builder, Handle};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub(crate) struct ServiceRuntime {
    pub tx: mpsc::Sender<ServiceCommand>,
    pub join: Option<std::thread::JoinHandle<()>>,
    pub handle: Handle,
}

static RUNTIME_SLOT: OnceLock<Mutex<Option<ServiceRuntime>>> = OnceLock::new();
static USER_AGENT_SLOT: OnceLock<Mutex<String>> = OnceLock::new();
const DEFAULT_USER_AGENT: &str = concat!("RongJS/", env!("CARGO_PKG_VERSION"));

pub(crate) fn runtime_slot() -> &'static Mutex<Option<ServiceRuntime>> {
    RUNTIME_SLOT.get_or_init(|| Mutex::new(None))
}

fn runtime_handle() -> Result<Handle, String> {
    let guard = runtime_slot().lock().unwrap();
    if let Some(rt) = guard.as_ref() {
        Ok(rt.handle.clone())
    } else {
        Err("service runtime not started".to_string())
    }
}

pub fn set_user_agent(ua: impl Into<String>) -> Result<(), String> {
    let ua_string = ua.into();
    HeaderValue::from_str(&ua_string).map_err(|e| format!("invalid user agent header: {}", e))?;
    let slot = USER_AGENT_SLOT.get_or_init(|| Mutex::new(DEFAULT_USER_AGENT.to_string()));
    let mut guard = slot.lock().unwrap();
    *guard = ua_string;
    Ok(())
}

pub fn get_user_agent() -> String {
    USER_AGENT_SLOT
        .get_or_init(|| Mutex::new(DEFAULT_USER_AGENT.to_string()))
        .lock()
        .unwrap()
        .clone()
}

pub fn start_service_runtime(worker_threads: usize) {
    let slot = runtime_slot();
    let mut guard = slot.lock().unwrap();
    if guard.is_some() {
        return;
    }

    let (tx, mut rx) = mpsc::channel::<ServiceCommand>(256);
    let rt = Builder::new_multi_thread()
        .worker_threads(worker_threads.max(1))
        .enable_all()
        .build()
        .expect("failed to build service runtime");

    let handle = rt.handle().clone();

    let join = thread::Builder::new()
        .name("rong-services".to_string())
        .spawn(move || {
            rt.block_on(async move {
                let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
                let https = HttpsConnectorBuilder::new()
                    .with_webpki_roots()
                    .https_or_http()
                    .enable_http1()
                    .build();
                let client: Client<_, http_body_util::combinators::BoxBody<Bytes, std::io::Error>> =
                    Client::builder(hyper_util::rt::TokioExecutor::new()).build(https);

                while let Some(cmd) = rx.recv().await {
                    match cmd {
                        ServiceCommand::Http(msg) => {
                            let client = client.clone();
                            tokio::task::spawn(async move {
                                net::process_request(client, msg).await;
                            });
                        }
                        ServiceCommand::Download {
                            url,
                            dest,
                            abort_rx,
                            sink,
                            completion,
                        } => {
                            tokio::task::spawn(async move {
                                let res =
                                    download::download_resource(&url, &dest, abort_rx, sink).await;
                                let _ = completion.send(res);
                            });
                        }
                        ServiceCommand::Shutdown(done_tx) => {
                            let _ = done_tx.send(());
                            break;
                        }
                    }
                }
            });
        })
        .expect("failed to spawn service runtime thread");

    *guard = Some(ServiceRuntime {
        tx,
        join: Some(join),
        handle,
    });
}

pub fn stop_service_runtime() {
    let slot = runtime_slot();
    let runtime = {
        let mut guard = slot.lock().unwrap();
        guard.take()
    };
    if let Some(mut rt) = runtime {
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        let _ = block_on(rt.tx.send(ServiceCommand::Shutdown(done_tx)));
        let _ = block_on(done_rx);
        if let Some(handle) = rt.join.take() {
            let _ = handle.join();
        }
    }
}

pub fn spawn_async<F, T>(future: F) -> Result<JoinHandle<T>, String>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let handle = runtime_handle()?;
    Ok(handle.spawn(future))
}

pub fn spawn_blocking<F, T>(func: F) -> Result<JoinHandle<T>, String>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let handle = runtime_handle()?;
    Ok(handle.spawn_blocking(func))
}
