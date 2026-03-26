//! Background Tokio runtime used for non-JS work (network, IO, blocking tasks).
//!
//! This runtime is process-global, shared across all worker threads and JS contexts.
//!
//! # Initialization
//!
//! Call [`init`] **before** the first spawn if you need custom configuration:
//!
//! ```rust,ignore
//! rong_rt::init(
//!     tokio::runtime::Builder::new_multi_thread()
//!         .worker_threads(8)
//!         .thread_name("my-bg")
//!         .enable_all(),
//! );
//! ```
//!
//! If [`init`] is never called, the runtime lazily starts on first use with
//! Tokio's defaults (`available_parallelism()` threads, all drivers enabled).

use std::future::Future;
use std::sync::OnceLock;

use tokio::runtime::{Handle, Runtime};
use tokio::task::JoinHandle;

/// The singleton runtime, created on first access.
static POOL: OnceLock<Runtime> = OnceLock::new();

fn default_runtime() -> Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .thread_name("rong-bg")
        .enable_all()
        .build()
        .expect("failed to build background runtime")
}

fn pool() -> &'static Runtime {
    POOL.get_or_init(default_runtime)
}

/// Optionally initialize the background runtime with a custom [`tokio::runtime::Builder`].
///
/// Returns `true` if the runtime was initialized, `false` if it was already
/// running (the builder is ignored in that case).
///
/// This is entirely optional — if never called, the first [`spawn`] or
/// [`handle`] call will create a runtime with Tokio's defaults.
pub fn init(builder: &mut tokio::runtime::Builder) -> bool {
    let runtime = builder.build().expect("failed to build background runtime");
    POOL.set(runtime).is_ok()
}

pub fn handle() -> Handle {
    pool().handle().clone()
}

pub fn spawn<F, T>(future: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    pool().spawn(future)
}

pub fn spawn_blocking<F, T>(func: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    pool().spawn_blocking(func)
}
