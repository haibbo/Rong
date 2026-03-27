//! Host-side Tokio executor used for non-JS work such as networking, file IO,
//! timers, and blocking helpers.
//!
//! The global executor is process-global and is used by `rong_rt` services.
//! Users can either install a custom global executor up front or rely on the
//! default global executor created on first use.

use std::future::Future;
use std::io;
use std::sync::{Arc, OnceLock};

use thiserror::Error;
use tokio::runtime::{Handle, Runtime};
use tokio::task::JoinHandle;

static GLOBAL_EXECUTOR: OnceLock<RongExecutor> = OnceLock::new();

/// Host-side executor for non-JS async work.
///
/// `RongExecutor` owns a Tokio multi-thread runtime used by `rong_rt` services
/// such as HTTP, download, upload, SSE, timers, and blocking helpers.
///
/// Most applications can either use the process-global executor via
/// [`RongExecutor::global`] or install a custom one once via
/// [`RongExecutor::install_global`].
#[derive(Clone)]
pub struct RongExecutor {
    runtime: Arc<Runtime>,
}

impl std::fmt::Debug for RongExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RongExecutor").finish_non_exhaustive()
    }
}

/// Builder for [`RongExecutor`].
///
/// The builder intentionally exposes only Rong-level knobs rather than the
/// full Tokio runtime builder surface.
#[derive(Debug, Clone)]
pub struct RongExecutorBuilder {
    threads: usize,
    thread_name: String,
}

/// Errors returned when constructing a [`RongExecutor`].
#[derive(Debug, Error)]
pub enum RongExecutorBuildError {
    #[error("executor threads must be greater than 0")]
    InvalidThreads,
    #[error("failed to build executor: {0}")]
    Build(#[from] io::Error),
}

/// Errors returned when installing the process-global executor.
#[derive(Debug, Error)]
pub enum InstallGlobalExecutorError {
    #[error("global RongExecutor is already installed")]
    AlreadyInstalled,
}

impl Default for RongExecutorBuilder {
    fn default() -> Self {
        Self {
            threads: std::thread::available_parallelism()
                .map(|count| count.get())
                .map(|count| count.min(4))
                .unwrap_or(1),
            thread_name: "rong-host".to_string(),
        }
    }
}

impl RongExecutorBuilder {
    /// Create a builder with Rong defaults.
    ///
    /// Defaults:
    /// - `threads = min(available_parallelism(), 4)`
    /// - `thread_name = "rong-host"`
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the number of worker threads used by the executor.
    ///
    /// Values less than `1` are rejected by [`RongExecutorBuilder::build`].
    pub fn threads(mut self, threads: usize) -> Self {
        self.threads = threads;
        self
    }

    /// Set the thread name prefix for executor worker threads.
    pub fn thread_name(mut self, name: impl Into<String>) -> Self {
        self.thread_name = name.into();
        self
    }

    /// Build a [`RongExecutor`] from this builder.
    pub fn build(self) -> Result<RongExecutor, RongExecutorBuildError> {
        if self.threads == 0 {
            return Err(RongExecutorBuildError::InvalidThreads);
        }

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(self.threads)
            .thread_name(self.thread_name)
            .enable_all()
            .build()?;

        Ok(RongExecutor {
            runtime: Arc::new(runtime),
        })
    }
}

impl RongExecutor {
    /// Create a builder for a custom host executor.
    pub fn builder() -> RongExecutorBuilder {
        RongExecutorBuilder::new()
    }

    /// Return the process-global executor.
    ///
    /// The first call lazily creates a default executor. Later calls return the
    /// same global instance.
    pub fn global() -> Self {
        GLOBAL_EXECUTOR
            .get_or_init(|| {
                RongExecutor::builder()
                    .build()
                    .expect("failed to build global RongExecutor")
            })
            .clone()
    }

    /// Install this executor as the process-global executor.
    ///
    /// This must happen before any code calls [`RongExecutor::global`] if you
    /// want the custom executor to become the global one.
    pub fn install_global(self) -> Result<(), InstallGlobalExecutorError> {
        GLOBAL_EXECUTOR
            .set(self)
            .map_err(|_| InstallGlobalExecutorError::AlreadyInstalled)
    }

    /// Return a clone of the underlying Tokio runtime handle.
    pub fn handle(&self) -> Handle {
        self.runtime.handle().clone()
    }

    /// Spawn a `Send + 'static` future onto this executor.
    pub fn spawn<F, T>(&self, future: F) -> JoinHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.runtime.spawn(future)
    }

    /// Run blocking work on this executor's blocking thread pool.
    pub fn spawn_blocking<F, T>(&self, func: F) -> JoinHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        self.runtime.spawn_blocking(func)
    }
}
