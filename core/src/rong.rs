use crate::{HostError, JSEngine, JSResult, JSRuntime};
use std::any::Any;
use std::cell::Cell;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::{Arc, Mutex as StdMutex};
use thiserror::Error;
use tokio::sync::{Mutex as TokioMutex, Notify, mpsc, oneshot};
use tracing::{Instrument, Span, debug, error, info, info_span, warn};

thread_local! {
    /// Set to `true` on worker threads to detect reentrant `block_on` / `get_worker_wait`.
    static INSIDE_WORKER: Cell<bool> = const { Cell::new(false) };
}

/// Worker states
///
/// Represents the current operational state of a worker in the thread pool.
#[derive(Clone, Debug, Copy, PartialEq)]
pub enum WorkerState {
    /// Worker is idle and ready to accept new tasks
    Free,
    /// Worker is currently executing a task
    Busy,
}

/// Represents messages intended for consumption by the user's asynchronous function
/// running within a worker, received via the `MessageReceiver`.
#[derive(Debug)]
pub enum WorkerMessage {
    String(String),
    Usize(usize),
    /// Container for any other user-defined message type.
    Custom(Box<dyn Any + Send>),
}

/// Message receiver for the user's asynchronous function to receive messages.
///
/// An instance of `MessageReceiver` is passed to the user-provided async function
/// when it's executed by a worker. It allows the function to receive messages
/// sent specifically to it via `Worker::post_message` (or its helpers).
pub struct MessageReceiver {
    /// Channel for receiving messages from the worker's broadcast channel
    receiver: mpsc::Receiver<WorkerMessage>,
}

impl MessageReceiver {
    /// Create a new message receiver from a channel
    fn new(receiver: mpsc::Receiver<WorkerMessage>) -> Self {
        Self { receiver }
    }

    /// Try to receive a message without blocking
    pub fn try_recv(&mut self) -> Result<WorkerMessage, mpsc::error::TryRecvError> {
        self.receiver.try_recv()
    }

    /// Receive a message asynchronously
    pub async fn recv(&mut self) -> Option<WorkerMessage> {
        self.receiver.recv().await
    }
}

// Type alias for the boxed future eventually produced by the closure in UserAsyncTask
type BoxedTaskFuture = Pin<Box<dyn Future<Output = JSResult<Box<dyn Any + Send>>>>>;

// Type alias for the boxed closure stored in UserAsyncTask
type BoxedFutureFn<E> =
    Box<dyn FnOnce(JSRuntime<<E as JSEngine>::Runtime>, MessageReceiver) -> BoxedTaskFuture + Send>;

// Type alias for the complex callback used in UserAsyncReturnType::BlockOn
type BlockOnCallback = Box<dyn FnOnce(JSResult<Box<dyn Any + Send>>) + Send>;

struct WorkerLoopContext {
    worker_id: usize,
    terminate_signal: Arc<tokio::sync::Notify>,
    state: Arc<TokioMutex<WorkerState>>,
    free_signal: Arc<Notify>,
    any_worker_free: Arc<Notify>,
    worker_span: Span,
    ready_tx: oneshot::Sender<Result<(), String>>,
}

/// Enum to differentiate how results are handled
enum UserAsyncReturnType {
    BlockOn(BlockOnCallback),
    Spawn, // No callback needed
}

/// Internal representation of a user-submitted asynchronous function submitted to a worker.
/// Holds the necessary components to invoke the user's future on the worker thread.
struct UserAsyncTask<E: JSEngine + 'static> {
    // Store the closure and receiver, not the final future, to avoid !Send issues with JSRuntime
    // The closure produces the boxed Any result type expected by result_tx
    future_fn: BoxedFutureFn<E>,
    message_receiver: MessageReceiver,
    parent_span: Span,

    /// Channel for the worker loop to forward post_message messages to this user's async function.
    task_message_tx: mpsc::Sender<WorkerMessage>,

    /// How to send the result back to the caller (or not).
    return_type: UserAsyncReturnType,
}

/// Worker - Individual JavaScript runtime worker
///
/// Represents a dedicated thread with the following characteristics:
/// - Runs a single user-provided asynchronous function at a time
/// - Reuses one JavaScript runtime per worker thread
/// - Supports message passing to the currently executing async function
/// - Maintains a state (Free/Busy) to indicate availability
/// - Has a signal for when the worker becomes free
///
/// `Worker` is cheaply cloneable (all fields are `Arc` or channel senders).
/// Clones share the same underlying thread — only dropping the last `Rong`
/// handle shuts workers down.
pub struct Worker<E: JSEngine + 'static> {
    /// Worker ID (index in the worker pool)
    id: usize,
    name: Option<String>,

    /// Channel for sending user async functions to the worker thread
    task_tx: mpsc::Sender<UserAsyncTask<E>>,

    /// Notify mechanism for signaling worker termination
    terminate_signal: Arc<tokio::sync::Notify>,

    /// Channel for sending messages to the current async function running on this worker
    /// Since a worker executes only one async function at a time, this is a simple MPSC channel
    message_tx: mpsc::Sender<WorkerMessage>,

    /// Worker state (Free/Busy)
    state: Arc<TokioMutex<WorkerState>>,

    /// Signal for when the worker becomes free
    free_signal: Arc<Notify>,

    /// Capacity for the per-task message channel created by `spawn` / `block_on`.
    message_queue_capacity: usize,

    /// Join handle for the dedicated worker thread.
    thread_handle: Arc<StdMutex<Option<std::thread::JoinHandle<()>>>>,
}

impl<E: JSEngine + 'static> Worker<E> {
    /// Set a custom name for this worker
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    /// Get the worker's ID
    pub fn id(&self) -> usize {
        self.id
    }

    /// Get the worker's name (or a default based on ID if not set)
    pub fn name(&self) -> String {
        self.name
            .clone()
            .unwrap_or_else(|| format!("worker-{}", self.id))
    }

    /// Get the worker's current state
    pub async fn state(&self) -> WorkerState {
        *self.state.lock().await
    }

    /// Spawn a user's asynchronous function on this worker
    ///
    /// Submits an asynchronous function to be executed on this worker's thread.
    /// The function will be executed on the worker's JavaScript thread and receives
    /// both the JSRuntime (as a reference) and a MessageReceiver for handling messages.
    ///
    /// This method returns immediately and does not wait for the async function to complete.
    /// The submitted function can access the JavaScript runtime and receive messages.
    pub fn spawn<F, Fut, R>(&self, future_fn: F) -> JSResult<()>
    where
        F: FnOnce(JSRuntime<E::Runtime>, MessageReceiver) -> Fut + Send + 'static,
        Fut: Future<Output = JSResult<R>> + 'static,
        R: Send + 'static,
    {
        // Prepare the future_fn that produces Box<dyn Any>
        let boxed_fn: BoxedFutureFn<E> = Box::new(
            move |runtime: JSRuntime<E::Runtime>, receiver: MessageReceiver| {
                let user_fut: Fut = future_fn(runtime, receiver);
                let user_fut_boxed = Box::pin(user_fut);
                let mapped_fut = async move {
                    user_fut_boxed
                        .await
                        .map(|r| Box::new(r) as Box<dyn Any + Send>)
                };
                Box::pin(mapped_fut) as BoxedTaskFuture
            },
        );

        // Setup message passing channels for this task.
        let (task_message_tx, task_message_rx) = mpsc::channel(self.message_queue_capacity);
        let message_receiver = MessageReceiver::new(task_message_rx);

        // Create task with Spawn mechanism
        let task = UserAsyncTask {
            future_fn: boxed_fn,
            message_receiver,
            parent_span: Span::current(),
            task_message_tx,
            return_type: UserAsyncReturnType::Spawn,
        };

        // Send task (non-blocking)
        if let Err(e) = self.task_tx.try_send(task) {
            error!(target: "rong", worker_id = self.id, error = ?e, "failed to queue worker task");
            return Err(HostError::new(
                crate::error::E_INTERNAL,
                format!(
                    "Failed to spawn future on worker {}: channel error: {:?}",
                    self.id, e
                ),
            )
            .into());
        }
        Ok(())
    }

    /// Execute a user's async function and wait for the result
    ///
    /// This is equivalent to `spawn` + join, but provides a synchronous interface.
    /// The method blocks until the async function completes and returns its result.
    /// Use this when you need to execute an async function and immediately use its return value.
    pub fn block_on<F, Fut, R>(&self, future_fn: F) -> JSResult<R>
    where
        F: FnOnce(JSRuntime<E::Runtime>, MessageReceiver) -> Fut + Send + 'static,
        Fut: Future<Output = JSResult<R>> + 'static,
        R: Send + 'static,
    {
        // Channel for the *final* R result (after downcast in the callback)
        let (final_result_tx, final_result_rx) = oneshot::channel::<JSResult<R>>();

        // Prepare the closure that handles the Box<dyn Any> result from the worker
        let result_callback = move |worker_result: JSResult<Box<dyn Any + Send>>| {
            let final_result = match worker_result {
                Ok(v_any) => match v_any.downcast::<R>() {
                    Ok(boxed_r) => Ok(*boxed_r),
                    Err(_) => Err(HostError::new(
                        crate::error::E_INTERNAL,
                        "Downcast failed in block_on callback",
                    )
                    .into()),
                },
                Err(e) => Err(e),
            };
            let _ = final_result_tx.send(final_result);
        };

        let return_type = UserAsyncReturnType::BlockOn(Box::new(result_callback));

        let boxed_fn: BoxedFutureFn<E> = Box::new(
            move |runtime: JSRuntime<E::Runtime>, receiver: MessageReceiver| {
                let user_fut: Fut = future_fn(runtime, receiver);
                let user_fut_boxed = Box::pin(user_fut);
                let mapped_fut = async move {
                    user_fut_boxed
                        .await
                        .map(|r| Box::new(r) as Box<dyn Any + Send>)
                };
                Box::pin(mapped_fut) as BoxedTaskFuture
            },
        );

        let (task_message_tx, task_message_rx) = mpsc::channel(self.message_queue_capacity);
        let message_receiver = MessageReceiver::new(task_message_rx);

        let task = UserAsyncTask {
            future_fn: boxed_fn,
            message_receiver,
            parent_span: Span::current(),
            task_message_tx,
            return_type,
        };

        // Send task to worker thread (blocking send)
        futures::executor::block_on(async {
            if let Err(e) = self.task_tx.send(task).await {
                return Err::<(), HostError>(HostError::new(
                    crate::error::E_INTERNAL,
                    format!("[block_on Worker {}] Failed to send task: {:?}", self.id, e),
                ));
            }
            Ok(())
        })?;

        // Wait for the final JSResult<R> from the callback
        futures::executor::block_on(async {
            final_result_rx.await.map_err(|e| {
                HostError::new(
                    crate::error::E_INTERNAL,
                    format!(
                        "[block_on Worker {}] Failed to receive final result: {:?}",
                        self.id, e
                    ),
                )
            })
        })?
    }

    /// Wait for this worker to complete its current async function
    ///
    /// Returns a future that resolves when the worker's state changes to Free.
    /// This can be awaited to ensure that a worker has finished processing before shutdown.
    pub async fn join(&self) -> JSResult<()> {
        loop {
            {
                let state_guard = self.state.lock().await;
                if *state_guard == WorkerState::Free {
                    return Ok(());
                }
            }

            // Wait for notification that state *might* be Free
            self.free_signal.notified().await;
        }
    }

    /// Ask the worker to terminate
    ///
    /// Sends a signal to gracefully stop the worker thread.
    /// Any running async functions will be interrupted and the worker thread will exit.
    pub fn terminate(&self) -> JSResult<()> {
        self.terminate_signal.notify_one();
        Ok(())
    }

    /// Post a message to this worker
    ///
    /// Sends a message to the currently executing async function on this worker.
    /// The running async function can receive this message through its MessageReceiver.
    ///
    /// If no async function is currently running, the message will be dropped.
    pub fn post_message(&self, message: WorkerMessage) -> JSResult<()> {
        self.message_tx.try_send(message).map_err(|e| {
            if matches!(e, mpsc::error::TrySendError::Full(_)) {
                warn!(
                    target: "rong",
                    worker_id = self.id,
                    "worker message channel full; dropping message"
                );
            } else if matches!(e, mpsc::error::TrySendError::Closed(_)) {
                warn!(target: "rong", worker_id = self.id, "worker message channel closed");
            }
            HostError::new(
                crate::error::E_INTERNAL,
                format!("Failed to post message to worker {}: {:?}", self.id, e),
            )
            .into()
        })
    }
}

/// Information about a worker
///
/// Contains details about a worker's identity and current state.
/// This is primarily used for monitoring and debugging.
pub struct WorkerInfo {
    /// Worker ID
    pub id: usize,
    /// Worker name
    pub name: String,
    /// Worker state
    pub state: WorkerState,
}

/// Errors returned when constructing a [`Rong`] worker pool.
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

/// Builder for [`Rong`] worker pools.
///
/// Provides a fluent interface for configuring how many JS workers are started
/// and how much queue capacity each worker receives.
pub struct RongBuilder<E: JSEngine + 'static> {
    workers: usize,
    task_queue_capacity: usize,
    message_queue_capacity: usize,
    _marker: PhantomData<E>,
}

impl<E: JSEngine + 'static> RongBuilder<E> {
    fn new() -> Self {
        Self {
            workers: 1,
            task_queue_capacity: 100,
            message_queue_capacity: 512,
            _marker: PhantomData,
        }
    }

    /// Set the number of JS worker threads. Defaults to 1.
    pub fn workers(mut self, count: usize) -> Self {
        self.workers = count;
        self
    }

    /// Set task queue capacity for each worker.
    ///
    /// Controls how many tasks can be queued per worker before backpressure
    /// occurs.
    pub fn task_queue_capacity(mut self, size: usize) -> Self {
        self.task_queue_capacity = size;
        self
    }

    /// Set message queue capacity for each worker.
    ///
    /// Controls how many messages can be buffered via [`Worker::post_message`]
    /// before backpressure or drops occur.
    pub fn message_queue_capacity(mut self, size: usize) -> Self {
        self.message_queue_capacity = size;
        self
    }

    /// Build and start a Rong worker pool.
    ///
    /// Initializes the worker pool. Host-side services use the shared
    /// `rong_rt::RongExecutor`.
    pub fn build(self) -> Result<Rong<E>, RongBuildError> {
        if self.workers == 0 {
            return Err(RongBuildError::InvalidWorkers);
        }
        if self.task_queue_capacity == 0 {
            return Err(RongBuildError::InvalidTaskQueueCapacity);
        }
        if self.message_queue_capacity == 0 {
            return Err(RongBuildError::InvalidMessageQueueCapacity);
        }

        let inner = Arc::new(RongInner {
            workers: Arc::new(TokioMutex::new(Vec::with_capacity(self.workers))),
            worker_count: self.workers,
            task_queue_capacity: self.task_queue_capacity,
            message_queue_capacity: self.message_queue_capacity,
            any_worker_free: Arc::new(Notify::new()),
        });

        if let Err(err) = inner.initialize_workers() {
            let rong = Rong {
                inner: inner.clone(),
            };
            let _ = rong.shutdown();
            return Err(err);
        }
        Ok(Rong { inner })
    }
}

struct RongInner<E: JSEngine + 'static> {
    workers: Arc<TokioMutex<Vec<Worker<E>>>>,
    worker_count: usize,
    task_queue_capacity: usize,
    message_queue_capacity: usize,
    /// Signalled (via `notify_one`) whenever any worker transitions to Free.
    /// `notify_one` stores a permit when no one is waiting, so no wake is lost.
    any_worker_free: Arc<Notify>,
}

/// Handle to a JS worker pool.
///
/// `Rong` is a cheap, cloneable handle around a shared pool of dedicated
/// worker threads. Each worker owns one JavaScript runtime and executes one
/// submitted task at a time.
///
/// The pool provides:
/// - Thread pool management for multiple JS runtimes
/// - Automatic task assignment to idle JS runtimes
/// - Efficient JavaScript execution avoiding frequent thread creation
/// - Message passing to running tasks
pub struct Rong<E: JSEngine + 'static> {
    inner: Arc<RongInner<E>>,
}

impl<E: JSEngine + 'static> Clone for Rong<E> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<E: JSEngine + 'static> Rong<E> {
    /// Create a new builder to configure and build a Rong worker pool.
    pub fn builder() -> RongBuilder<E> {
        RongBuilder::new()
    }

    /// Execute a task on the next available worker and wait for the result.
    ///
    /// This is the high-level convenience entry point when you do not need to
    /// manually acquire a specific [`Worker`].
    pub fn block_on<F, Fut, R>(&self, future_fn: F) -> JSResult<R>
    where
        F: FnOnce(JSRuntime<E::Runtime>, MessageReceiver) -> Fut + Send + 'static,
        Fut: Future<Output = JSResult<R>> + 'static,
        R: Send + 'static,
    {
        let worker = futures::executor::block_on(self.get_worker_wait())?;
        worker.block_on(future_fn)
    }
}

impl<E: JSEngine + 'static> RongInner<E> {
    /// Initialize the worker pool and wait until every worker is ready.
    fn initialize_workers(self: &Arc<Self>) -> Result<(), RongBuildError> {
        let mut ready_receivers = Vec::with_capacity(self.worker_count);

        futures::executor::block_on(async {
            let mut workers_guard = self.workers.lock().await;

            for i in 0..self.worker_count {
                let (task_tx, task_rx) = mpsc::channel(self.task_queue_capacity);
                let terminate_signal = Arc::new(Notify::new());
                let (worker_message_tx, worker_message_rx) =
                    mpsc::channel(self.message_queue_capacity);

                let state = Arc::new(TokioMutex::new(WorkerState::Free));
                let free_signal = Arc::new(Notify::new());
                let thread_handle = Arc::new(StdMutex::new(None));

                let worker = Worker {
                    id: i,
                    name: None,
                    task_tx: task_tx.clone(),
                    terminate_signal: terminate_signal.clone(),
                    message_tx: worker_message_tx,
                    state: state.clone(),
                    free_signal: free_signal.clone(),
                    message_queue_capacity: self.message_queue_capacity,
                    thread_handle: thread_handle.clone(),
                };

                workers_guard.push(worker);

                let state_clone = state.clone();
                let free_signal_clone = free_signal.clone();
                let any_free_clone = self.any_worker_free.clone();
                let worker_span = info_span!("rong.worker", worker_id = i);

                let (ready_tx, ready_rx) = oneshot::channel::<Result<(), String>>();
                ready_receivers.push((i, ready_rx));

                let handle = std::thread::spawn(move || {
                    let _worker_entered = worker_span.enter();
                    info!(target: "rong", "worker thread started");
                    INSIDE_WORKER.with(|flag| flag.set(true));

                    let rt = match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .thread_name(format!("worker-{}", i))
                        .build()
                    {
                        Ok(rt) => rt,
                        Err(err) => {
                            let _ = ready_tx.send(Err(err.to_string()));
                            return;
                        }
                    };

                    rt.block_on(async {
                        Self::run_worker_loop(
                            WorkerLoopContext {
                                worker_id: i,
                                terminate_signal,
                                state: state_clone,
                                free_signal: free_signal_clone,
                                any_worker_free: any_free_clone,
                                worker_span: worker_span.clone(),
                                ready_tx,
                            },
                            task_rx,
                            worker_message_rx,
                        )
                        .await;
                    });
                    info!(target: "rong", "worker thread stopped");
                });
                *thread_handle.lock().unwrap() = Some(handle);
            }
        });

        // Wait for all worker threads to finish JS runtime initialization
        // before returning, so workers are truly ready to accept tasks.
        for (worker_id, rx) in ready_receivers {
            match futures::executor::block_on(rx) {
                Ok(Ok(())) => {}
                Ok(Err(reason)) => {
                    return Err(RongBuildError::WorkerStart { worker_id, reason });
                }
                Err(err) => {
                    return Err(RongBuildError::WorkerStart {
                        worker_id,
                        reason: err.to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Core processing loop for a worker thread.
    async fn run_worker_loop(
        ctx: WorkerLoopContext,
        mut task_rx: mpsc::Receiver<UserAsyncTask<E>>,
        mut worker_message_rx: mpsc::Receiver<WorkerMessage>,
    ) {
        let WorkerLoopContext {
            worker_id,
            terminate_signal,
            state,
            free_signal,
            any_worker_free,
            worker_span,
            ready_tx,
        } = ctx;
        let local = tokio::task::LocalSet::new();

        local
            .run_until(async move {
                let mut should_terminate = false;

                // Create the JS runtime once per worker thread and reuse it
                // across all tasks. This avoids the expensive per-task runtime
                // creation (e.g. JSContextGroupCreate can take seconds in debug
                // builds for JavaScriptCore).
                let js_runtime = E::runtime();

                // Signal that this worker is ready to accept tasks.
                let _ = ready_tx.send(Ok(()));

                // Start the microtask polling runner for engines that need it.
                // The runner lives for the lifetime of the worker, not per-task.
                let _microtask_runner_handle: Option<tokio::task::JoinHandle<()>> =
                    if js_runtime.run_pending_jobs() >= 0 {
                        let rt_clone = js_runtime.clone();
                        let microtask_span = info_span!(
                            parent: &worker_span,
                            "rong.microtasks",
                            worker_id = worker_id
                        );
                        Some(spawn_local(
                            async move {
                                let mut interval = tokio::time::interval(
                                    std::time::Duration::from_millis(50),
                                );
                                loop {
                                    interval.tick().await;
                                    rt_clone.run_pending_jobs();
                                }
                            }
                            .instrument(microtask_span),
                        ))
                    } else {
                        None
                    };

                type TaskJoinHandle = tokio::task::JoinHandle<
                    Result<JSResult<Box<dyn Any + Send>>, futures::future::Aborted>,
                >;
                let mut current_task_join_handle: Option<TaskJoinHandle> = None;
                let mut current_task_abort_handle: Option<futures::future::AbortHandle> = None;
                let mut current_task_message_tx: Option<mpsc::Sender<WorkerMessage>> = None;
                let mut current_task_result_callback: Option<BlockOnCallback> = None;
                let mut current_task_span: Option<Span> = None;

                while !should_terminate {
                    tokio::select! {
                        biased;

                        // Process termination signal
                        _ = terminate_signal.notified() => {
                            if let Some(handle) = current_task_abort_handle.take() {
                                handle.abort();
                            }
                            should_terminate = true;
                        },

                        // Process new user async functions, only if no task is currently running
                        maybe_task = task_rx.recv(), if current_task_join_handle.is_none() && !should_terminate => {
                            if let Some(user_async_task) = maybe_task {
                                {
                                    let mut state_guard = state.lock().await;
                                    *state_guard = WorkerState::Busy;
                                }

                                let task_span = info_span!(
                                    parent: &user_async_task.parent_span,
                                    "rong.task",
                                    worker_id = worker_id
                                );
                                debug!(target: "rong", parent: &task_span, "worker task started");
                                current_task_span = Some(task_span.clone());

                                current_task_message_tx = Some(user_async_task.task_message_tx);
                                match user_async_task.return_type {
                                    UserAsyncReturnType::BlockOn(callback) => {
                                        current_task_result_callback = Some(callback);
                                    }
                                    UserAsyncReturnType::Spawn => {
                                        current_task_result_callback = None;
                                    }
                                }

                                let user_fn = user_async_task.future_fn;
                                let message_receiver = user_async_task.message_receiver;
                                let user_future =
                                    user_fn(js_runtime.clone(), message_receiver).instrument(task_span.clone());

                                let (abortable_future, abort_handle) = futures::future::abortable(user_future);
                                current_task_abort_handle = Some(abort_handle);

                                let task_handle =
                                    spawn_local(abortable_future.instrument(task_span.clone()));
                                current_task_join_handle = Some(task_handle);

                            } else {
                                // task_rx closed
                                should_terminate = true;
                            }
                        },

                        // Process messages for the currently running task
                        maybe_message = worker_message_rx.recv(), if current_task_message_tx.is_some() => {
                             if let Some(message) = maybe_message
                                && let Some(tx) = &current_task_message_tx
                                && tx.send(message).await.is_ok() {
                                    js_runtime.run_pending_jobs();
                             }
                             // If worker_message_rx closed, let running task finish
                        },

                        // Wait for the current user task to complete
                        maybe_result = async { current_task_join_handle.as_mut().unwrap().await }, if current_task_join_handle.is_some() => {
                            let final_result: JSResult<Box<dyn Any + Send>> = match maybe_result {
                                Ok(Ok(inner_result)) => inner_result,
                                Ok(Err(_aborted)) => Err(HostError::aborted(None).into()),
                                Err(join_error) => {
                                     if let Some(task_span) = current_task_span.as_ref() {
                                         error!(target: "rong", parent: task_span, worker_id = worker_id, error = ?join_error, "user task panicked or runtime dropped");
                                     } else {
                                         error!(target: "rong", parent: &worker_span, worker_id = worker_id, error = ?join_error, "user task panicked or runtime dropped");
                                     }
                                     Err(HostError::new(crate::error::E_INTERNAL, format!("User task panicked or runtime dropped: {}", join_error)).into())
                                }
                            };

                            if let Some(callback) = current_task_result_callback.take() {
                                 callback(final_result);
                            } else if let Err(e) = final_result {
                                 if let Some(task_span) = current_task_span.as_ref() {
                                     error!(target: "rong", parent: task_span, worker_id = worker_id, error = ?e, "spawned task failed");
                                 } else {
                                     error!(target: "rong", parent: &worker_span, worker_id = worker_id, error = ?e, "spawned task failed");
                                 }
                            } else if let Some(task_span) = current_task_span.as_ref() {
                                 debug!(target: "rong", parent: task_span, "worker task completed");
                            }

                            // Clean up task state
                            current_task_join_handle = None;
                            current_task_abort_handle = None;
                            current_task_message_tx = None;
                            current_task_span = None;

                            // Set worker state back to Free
                            {
                                let mut state_guard = state.lock().await;
                                *state_guard = WorkerState::Free;
                                free_signal.notify_waiters();
                                any_worker_free.notify_one();
                            }
                        },
                    }
                }

                // Final cleanup if terminated while task was running
                if let Some(handle) = current_task_abort_handle.take() {
                     handle.abort();
                }
                if let Some(handle) = _microtask_runner_handle {
                     handle.abort();
                }

                let _ = current_task_result_callback.take();

                {
                    let mut state_guard = state.lock().await;
                    *state_guard = WorkerState::Free;
                    free_signal.notify_waiters();
                    any_worker_free.notify_one();
                }
            })
            .await;
    }
}

impl<E: JSEngine + 'static> Rong<E> {
    /// Try to acquire a free worker immediately.
    ///
    /// Returns an error when all workers are currently busy.
    pub async fn get_worker(&self) -> JSResult<Worker<E>> {
        let workers_guard = self.inner.workers.lock().await;

        for worker in workers_guard.iter() {
            let mut state_guard = worker.state.lock().await;

            if *state_guard == WorkerState::Free {
                *state_guard = WorkerState::Busy;
                drop(state_guard);
                return Ok(worker.clone());
            }
        }

        Err(HostError::new(crate::error::E_INVALID_STATE, "No free worker available").into())
    }

    /// Get a free worker, waiting if all are currently busy.
    ///
    /// Waits for any worker to become free, then returns it.
    ///
    /// # Panics / Errors
    /// Must NOT be called from within a worker thread (reentrant `block_on`).
    /// Doing so will deadlock when `worker_count == 1` and risks starvation otherwise.
    pub async fn get_worker_wait(&self) -> JSResult<Worker<E>> {
        // Prevent reentrant deadlock: if we're already on a worker thread,
        // fail immediately instead of waiting forever.
        if INSIDE_WORKER.with(|flag| flag.get()) {
            return Err(HostError::new(
                crate::error::E_INTERNAL,
                "get_worker_wait() called from inside a worker thread (reentrant block_on); \
                 this would deadlock",
            )
            .into());
        }

        loop {
            // Try to grab a free worker.
            if let Ok(w) = self.get_worker().await {
                return Ok(w);
            }

            // No free worker — wait for any worker to become free.
            // `notify_one` stores a permit when no one is waiting, so a wake
            // that fires between get_worker() and this await is not lost.
            self.inner.any_worker_free.notified().await;
        }
    }

    /// Return the number of workers currently in the `Free` state.
    pub async fn free_workers_count(&self) -> usize {
        let workers = self.inner.workers.lock().await;
        let mut count = 0;
        for w in workers.iter() {
            if *w.state.lock().await == WorkerState::Free {
                count += 1;
            }
        }
        count
    }

    /// Return the total number of workers in the pool.
    pub async fn total_workers_count(&self) -> usize {
        let workers = self.inner.workers.lock().await;
        workers.len()
    }

    /// Wait until all workers in the pool become free.
    pub async fn join_all(&self) -> JSResult<()> {
        let workers_guard = self.inner.workers.lock().await;
        let workers_to_join = workers_guard.iter().cloned().collect::<Vec<_>>();
        drop(workers_guard);

        let join_futures = workers_to_join.iter().map(|w| w.join());

        match futures::future::try_join_all(join_futures).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Shutdown all workers.
    ///
    /// Sends termination signals to all workers, regardless of their state.
    fn shutdown(&self) -> JSResult<()> {
        let thread_handles = futures::executor::block_on(async {
            let workers = self.inner.workers.lock().await;
            let mut thread_handles = Vec::with_capacity(workers.len());
            for worker in workers.iter() {
                if let Err(e) = worker.terminate() {
                    warn!(target: "rong", worker_id = worker.id, error = ?e, "failed to terminate worker");
                }
                thread_handles.push((worker.id, worker.thread_handle.clone()));
            }
            thread_handles
        });

        for (worker_id, thread_handle) in thread_handles {
            let handle = {
                let mut guard = thread_handle.lock().unwrap();
                guard.take()
            };

            if let Some(handle) = handle {
                if handle.thread().id() == std::thread::current().id() {
                    warn!(
                        target: "rong",
                        worker_id,
                        "skipping join on current worker thread during shutdown"
                    );
                    continue;
                }

                if let Err(err) = handle.join() {
                    warn!(
                        target: "rong",
                        worker_id,
                        error = ?err,
                        "worker thread panicked during shutdown"
                    );
                }
            }
        }

        Ok(())
    }
}

impl<E: JSEngine + 'static> Drop for Rong<E> {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) == 1 {
            let _ = self.shutdown();
        }
    }
}

/// Spawn a task onto the current thread's `LocalSet`.
///
/// This helper is for work that must stay on the current JS worker thread. It
/// does not use [`rong_rt::RongExecutor`].
pub fn spawn_local<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: Future + 'static,
{
    tokio::task::spawn_local(future)
}

impl<E: JSEngine + 'static> Clone for Worker<E> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            task_tx: self.task_tx.clone(),
            terminate_signal: self.terminate_signal.clone(),
            message_tx: self.message_tx.clone(),
            state: self.state.clone(),
            free_signal: self.free_signal.clone(),
            message_queue_capacity: self.message_queue_capacity,
            thread_handle: self.thread_handle.clone(),
        }
    }
}

// NOTE: Worker::Drop intentionally does NOT send terminate signals.
// Worker is Clone — every clone would kill the shared thread on drop.
// Only dropping the last `Rong` handle shuts worker threads down.
