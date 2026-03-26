use crate::{HostError, JSEngine, JSResult, JSRuntime};
use std::any::Any;
use std::cell::Cell;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
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
    ready_tx: oneshot::Sender<()>,
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
/// - Creates a fresh JavaScript runtime for each user function to ensure isolation
/// - Supports message passing to the currently executing async function
/// - Maintains a state (Free/Busy) to indicate availability
/// - Has a signal for when the worker becomes free
///
/// `Worker` is cheaply cloneable (all fields are `Arc` or channel senders).
/// Clones share the same underlying thread — only `Rong::shutdown` terminates threads.
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

    /// Parent Rong instance
    rong: Arc<Rong<E>>,
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
    pub fn spawn_future<F, Fut, R>(&self, future_fn: F) -> JSResult<()>
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
        let (task_message_tx, task_message_rx) = mpsc::channel(self.rong.message_queue_size);
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
    /// This is equivalent to spawn_future + join, but provides a synchronous interface.
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

        let (task_message_tx, task_message_rx) = mpsc::channel(self.rong.message_queue_size);
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

/// Builder for Rong instances
///
/// Provides a fluent interface for configuring and creating Rong instances
/// with customized worker pools and queue sizes.
pub struct RongBuilder<E: JSEngine + 'static> {
    worker_count: usize,
    task_queue_size: usize,
    message_queue_size: usize,
    _marker: PhantomData<E>,
}

impl<E: JSEngine + 'static> RongBuilder<E> {
    fn new() -> Self {
        Self {
            worker_count: 1,
            task_queue_size: 100,
            message_queue_size: 512,
            _marker: PhantomData,
        }
    }

    /// Set the number of JS worker threads. Defaults to 1.
    pub fn with_num_workers(mut self, count: usize) -> Self {
        assert!(count >= 1, "At least one worker thread is required");
        self.worker_count = count;
        self
    }

    /// Set task queue size for each worker.
    ///
    /// Controls how many tasks can be queued per worker before backpressure occurs.
    pub fn with_task_queue_size(mut self, size: usize) -> Self {
        self.task_queue_size = size;
        self
    }

    /// Set message queue size for each worker.
    ///
    /// Controls how many messages can be buffered via `post_message` before dropping.
    pub fn with_message_queue_size(mut self, size: usize) -> Self {
        self.message_queue_size = size;
        self
    }

    /// Build and start a Rong instance.
    ///
    /// Initializes the worker pool. The background I/O runtime (`rong_rt`) is
    /// lazily started on first use with `available_parallelism()` threads.
    pub fn build(self) -> Arc<Rong<E>> {
        let rong = Arc::new(Rong {
            workers: Arc::new(TokioMutex::new(Vec::with_capacity(self.worker_count))),
            worker_count: self.worker_count,
            task_queue_size: self.task_queue_size,
            message_queue_size: self.message_queue_size,
            any_worker_free: Arc::new(Notify::new()),
        });

        rong.initialize_workers();
        rong
    }
}

/// Rong - JS runtime container manager
///
/// Thread pool manager for JavaScript runtimes. Provides:
/// - Thread pool management for multiple JS runtimes
/// - Automatic task assignment to idle JS runtimes
/// - Efficient JavaScript execution avoiding frequent thread creation
/// - Message passing to running tasks
///
/// Each worker in the pool runs in its own dedicated thread with its own
/// JavaScript runtime, ensuring isolation and thread safety.
pub struct Rong<E: JSEngine + 'static> {
    workers: Arc<TokioMutex<Vec<Worker<E>>>>,
    worker_count: usize,
    task_queue_size: usize,
    message_queue_size: usize,
    /// Signalled (via `notify_one`) whenever any worker transitions to Free.
    /// `notify_one` stores a permit when no one is waiting, so no wake is lost.
    any_worker_free: Arc<Notify>,
}

impl<E: JSEngine + 'static> Rong<E> {
    /// Create a new builder to configure and build a Rong instance.
    pub fn builder() -> RongBuilder<E> {
        RongBuilder::new()
    }

    /// Execute a user's async function and wait for the result.
    ///
    /// Automatically gets a free worker (waiting if necessary) and executes the
    /// async function on it, blocking until the function completes.
    pub fn block_on<F, Fut, R>(&self, future_fn: F) -> JSResult<R>
    where
        F: FnOnce(JSRuntime<E::Runtime>, MessageReceiver) -> Fut + Send + 'static,
        Fut: Future<Output = JSResult<R>> + 'static,
        R: Send + 'static,
    {
        let worker = futures::executor::block_on(self.get_worker_wait())?;
        worker.block_on(future_fn)
    }

    /// Initialize the worker pool
    fn initialize_workers(self: &Arc<Self>) {
        let mut ready_receivers = Vec::with_capacity(self.worker_count);

        futures::executor::block_on(async {
            let mut workers_guard = self.workers.lock().await;

            for i in 0..self.worker_count {
                let (task_tx, task_rx) = mpsc::channel(self.task_queue_size);
                let terminate_signal = Arc::new(Notify::new());
                let (worker_message_tx, worker_message_rx) = mpsc::channel(self.message_queue_size);

                let state = Arc::new(TokioMutex::new(WorkerState::Free));
                let free_signal = Arc::new(Notify::new());

                let worker = Worker {
                    id: i,
                    name: None,
                    task_tx: task_tx.clone(),
                    terminate_signal: terminate_signal.clone(),
                    message_tx: worker_message_tx,
                    state: state.clone(),
                    free_signal: free_signal.clone(),
                    rong: self.clone(),
                };

                workers_guard.push(worker);

                let state_clone = state.clone();
                let free_signal_clone = free_signal.clone();
                let any_free_clone = self.any_worker_free.clone();
                let worker_span = info_span!("rong.worker", worker_id = i);

                let (ready_tx, ready_rx) = oneshot::channel::<()>();
                ready_receivers.push(ready_rx);

                std::thread::spawn(move || {
                    let _worker_entered = worker_span.enter();
                    info!(target: "rong", "worker thread started");
                    INSIDE_WORKER.with(|flag| flag.set(true));

                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .thread_name(format!("worker-{}", i))
                        .build()
                        .expect("Failed to create worker runtime");

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
            }
        });

        // Wait for all worker threads to finish JS runtime initialization
        // before returning, so workers are truly ready to accept tasks.
        for rx in ready_receivers {
            let _ = futures::executor::block_on(rx);
        }
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
                let _ = ready_tx.send(());

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
                        Some(spawn(
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

                                let task_handle = spawn(abortable_future.instrument(task_span.clone()));
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

    /// Try to get a free worker immediately. Returns an error if none are free.
    pub async fn get_worker(&self) -> JSResult<Worker<E>> {
        let workers_guard = self.workers.lock().await;

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
            self.any_worker_free.notified().await;
        }
    }

    /// Get the count of free workers in the pool
    pub async fn free_workers_count(&self) -> usize {
        let workers = self.workers.lock().await;
        let mut count = 0;
        for w in workers.iter() {
            if *w.state.lock().await == WorkerState::Free {
                count += 1;
            }
        }
        count
    }

    /// Get total number of workers in the pool
    pub async fn total_workers_count(&self) -> usize {
        let workers = self.workers.lock().await;
        workers.len()
    }

    /// Wait for all workers to become free
    pub async fn join_all(&self) -> JSResult<()> {
        let workers_guard = self.workers.lock().await;
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
        futures::executor::block_on(async {
            let workers = self.workers.lock().await;
            for worker in workers.iter() {
                if let Err(e) = worker.terminate() {
                    warn!(target: "rong", worker_id = worker.id, error = ?e, "failed to terminate worker");
                }
            }
        });
        Ok(())
    }
}

impl<E: JSEngine + 'static> Drop for Rong<E> {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

/// Spawn a local async task
///
/// Convenience wrapper around `tokio::task::spawn_local`.
pub fn spawn<F>(future: F) -> tokio::task::JoinHandle<F::Output>
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
            rong: self.rong.clone(),
        }
    }
}

// NOTE: Worker::Drop intentionally does NOT send terminate signals.
// Worker is Clone — every clone would kill the shared thread on drop.
// Only Rong::shutdown (called from Rong::Drop) terminates worker threads.
