//! Background Tokio runtime used for non-JS work (network, IO, blocking tasks).
//!
//! This runtime is process-global so it can be shared across all worker threads/JS contexts.
//! It is configured by the first call to `start()`, so prefer initializing it via
//! `Rong::builder().with_service_threads(...).build()` early in program startup.
//!
//! The runtime persists for the lifetime of the process unless explicitly stopped via `stop()`.

use std::future::Future;
use std::sync::{LazyLock, RwLock};
use std::time::Duration;

use tokio::runtime::{Handle, Runtime};
use tokio::task::JoinHandle;

#[derive(Debug)]
struct BgPool {
    worker_threads: usize,
    runtime: Runtime,
}

static POOL: LazyLock<RwLock<Option<BgPool>>> = LazyLock::new(|| RwLock::new(None));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BgPoolError {
    NotStarted,
}

impl std::fmt::Display for BgPoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotStarted => write!(f, "background pool not started"),
        }
    }
}

impl std::error::Error for BgPoolError {}

fn build_runtime(worker_threads: usize) -> Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads.max(1))
        .thread_name("rong-bg")
        .enable_all()
        .build()
        .expect("failed to build background runtime")
}

pub fn start(worker_threads: usize) {
    let worker_threads = worker_threads.max(1);
    let mut guard = POOL.write().unwrap();
    if let Some(existing) = guard.as_ref() {
        if existing.worker_threads != worker_threads {
            panic!(
                "background pool already started with {} threads (requested {})",
                existing.worker_threads, worker_threads
            );
        }
        return;
    }
    *guard = Some(BgPool {
        worker_threads,
        runtime: build_runtime(worker_threads),
    });
}

pub fn stop() {
    let pool = POOL.write().unwrap().take();
    if let Some(pool) = pool {
        // Give tasks a chance to finish, but don't block indefinitely.
        pool.runtime.shutdown_timeout(Duration::from_secs(1));
    }
}

pub fn is_started() -> bool {
    POOL.read().unwrap().is_some()
}

pub fn handle() -> Option<Handle> {
    POOL.read()
        .unwrap()
        .as_ref()
        .map(|p| p.runtime.handle().clone())
}

pub fn spawn<F, T>(future: F) -> Result<JoinHandle<T>, BgPoolError>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let guard = POOL.read().unwrap();
    let pool = guard.as_ref().ok_or(BgPoolError::NotStarted)?;
    Ok(pool.runtime.spawn(future))
}

pub fn spawn_blocking<F, T>(func: F) -> Result<JoinHandle<T>, BgPoolError>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let guard = POOL.read().unwrap();
    let pool = guard.as_ref().ok_or(BgPoolError::NotStarted)?;
    Ok(pool.runtime.spawn_blocking(func))
}
