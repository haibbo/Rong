use crate::pinned::PinnedRong;
use crate::shared::build_shared_workers;
use crate::{HostError, JSEngine};
use std::marker::PhantomData;
use thiserror::Error;

pub use crate::shared::Rong;
pub use crate::shared::{
    MessageReceiver, TaskHandle, TaskMessage, Worker, WorkerState, spawn_local,
};

#[derive(Debug, Error)]
pub enum RongBuildError {
    #[error("workers must be greater than 0")]
    InvalidWorkers,
    #[error("task queue capacity must be greater than 0")]
    InvalidTaskQueueCapacity,
    #[error("message queue capacity must be greater than 0")]
    InvalidMessageQueueCapacity,
    #[error("worker {worker_id} failed to start: {reason}")]
    WorkerStart { worker_id: usize, reason: String },
}

impl From<RongBuildError> for crate::RongJSError {
    fn from(value: RongBuildError) -> Self {
        HostError::new(crate::error::E_INTERNAL, value.to_string()).into()
    }
}

/// Selects which Rong worker-pool model to build.
///
/// `RongBuilder` is intentionally not buildable on its own. Call either
/// [`RongBuilder::shared`] or [`RongBuilder::pinned`] first to make the
/// execution model explicit, then finish with `.build()`.
pub struct RongBuilder<E: JSEngine + 'static> {
    config: RongBuilderConfig,
    _marker: PhantomData<E>,
}

/// Builder for the shared worker-pool model.
///
/// Shared pools maximize throughput for stateless work: each task is routed to
/// any available worker and should not assume affinity to a prior invocation.
pub struct SharedRongBuilder<E: JSEngine + 'static> {
    config: RongBuilderConfig,
    _marker: PhantomData<E>,
}

/// Builder for the pinned worker-pool model.
///
/// Pinned pools keep the same key on the same long-lived worker so per-key
/// state can be reused across invocations.
pub struct PinnedRongBuilder<E: JSEngine + 'static, K, S> {
    config: RongBuilderConfig,
    _engine: PhantomData<E>,
    _key: PhantomData<K>,
    _state: PhantomData<S>,
}

#[derive(Clone, Copy)]
struct RongBuilderConfig {
    workers: usize,
    task_queue_capacity: usize,
    message_queue_capacity: usize,
}

impl Default for RongBuilderConfig {
    fn default() -> Self {
        Self {
            workers: 1,
            task_queue_capacity: 100,
            message_queue_capacity: 512,
        }
    }
}

impl RongBuilderConfig {
    fn validate(self) -> Result<Self, RongBuildError> {
        if self.workers == 0 {
            return Err(RongBuildError::InvalidWorkers);
        }
        if self.task_queue_capacity == 0 {
            return Err(RongBuildError::InvalidTaskQueueCapacity);
        }
        if self.message_queue_capacity == 0 {
            return Err(RongBuildError::InvalidMessageQueueCapacity);
        }
        Ok(self)
    }
}

impl<E: JSEngine + 'static> RongBuilder<E> {
    fn new() -> Self {
        Self {
            config: RongBuilderConfig::default(),
            _marker: PhantomData,
        }
    }

    pub fn workers(mut self, count: usize) -> Self {
        self.config.workers = count;
        self
    }

    pub fn task_queue_capacity(mut self, size: usize) -> Self {
        self.config.task_queue_capacity = size;
        self
    }

    pub fn message_queue_capacity(mut self, size: usize) -> Self {
        self.config.message_queue_capacity = size;
        self
    }

    /// Choose the shared worker-pool model.
    ///
    /// Use this when tasks are independent and do not need affinity to prior
    /// runs. The resulting pool dispatches each task to any available worker.
    pub fn shared(self) -> SharedRongBuilder<E> {
        SharedRongBuilder {
            config: self.config,
            _marker: PhantomData,
        }
    }

    /// Choose the pinned worker-pool model.
    ///
    /// Use this when the same routing key must keep landing on the same
    /// long-lived worker, typically to reuse per-key state.
    pub fn pinned<K, S>(self) -> PinnedRongBuilder<E, K, S>
    where
        K: Eq + std::hash::Hash + Clone + Send + 'static,
        S: 'static,
    {
        PinnedRongBuilder {
            config: self.config,
            _engine: PhantomData,
            _key: PhantomData,
            _state: PhantomData,
        }
    }
}

impl<E: JSEngine + 'static> SharedRongBuilder<E> {
    pub fn workers(mut self, count: usize) -> Self {
        self.config.workers = count;
        self
    }

    pub fn task_queue_capacity(mut self, size: usize) -> Self {
        self.config.task_queue_capacity = size;
        self
    }

    pub fn message_queue_capacity(mut self, size: usize) -> Self {
        self.config.message_queue_capacity = size;
        self
    }

    pub fn build(self) -> Result<Rong<E>, RongBuildError> {
        let config = self.config.validate()?;
        build_shared_workers::<E>(
            config.workers,
            config.task_queue_capacity,
            config.message_queue_capacity,
        )
    }
}

impl<E, K, S> PinnedRongBuilder<E, K, S>
where
    E: JSEngine + 'static,
    K: Eq + std::hash::Hash + Clone + Send + 'static,
    S: 'static,
{
    pub fn workers(mut self, count: usize) -> Self {
        self.config.workers = count;
        self
    }

    pub fn task_queue_capacity(mut self, size: usize) -> Self {
        self.config.task_queue_capacity = size;
        self
    }

    pub fn message_queue_capacity(mut self, size: usize) -> Self {
        self.config.message_queue_capacity = size;
        self
    }

    pub fn build(self) -> Result<PinnedRong<E, K, S>, RongBuildError> {
        let config = self.config.validate()?;
        PinnedRong::build(
            config.workers,
            config.task_queue_capacity,
            config.message_queue_capacity,
        )
    }
}

impl<E: JSEngine + 'static> Rong<E> {
    /// Start building a Rong worker pool.
    ///
    /// Pick an execution model explicitly:
    ///
    /// - [`RongBuilder::shared`] for stateless work dispatched to any idle worker.
    /// - [`RongBuilder::pinned`] for keyed work that must reuse a long-lived worker.
    pub fn builder() -> RongBuilder<E> {
        RongBuilder::new()
    }
}
