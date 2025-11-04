use crate::{JSEngine, JSResult, JSRuntime, RongJSError};
use std::any::Any;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{Mutex as TokioMutex, Notify, mpsc, oneshot};

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
                // 1. Call user's function to get the anonymous Future `Fut`
                let user_fut: Fut = future_fn(runtime, receiver);
                // 2. Box and Pin it *immediately* for type erasure
                let user_fut_boxed = Box::pin(user_fut);
                // 3. Create the mapping future that awaits the boxed future
                //    and converts the result R to Box<dyn Any + Send>
                let mapped_fut = async move {
                    user_fut_boxed
                        .await
                        .map(|r| Box::new(r) as Box<dyn Any + Send>)
                };
                // 4. Return the pinned, boxed mapping future, cast to BoxedTaskFuture
                Box::pin(mapped_fut) as BoxedTaskFuture
            },
        );

        // Setup message passing channels for this task
        let (task_message_tx, task_message_rx) = mpsc::channel(100);
        let message_receiver = MessageReceiver::new(task_message_rx);

        // Create task with Spawn mechanism
        let task = UserAsyncTask {
            future_fn: boxed_fn,
            message_receiver,
            task_message_tx,
            return_type: UserAsyncReturnType::Spawn, // Indicate no result needed back
        };

        // Send task (non-blocking)
        if let Err(e) = self.task_tx.try_send(task) {
            eprintln!(
                "[spawn_future Worker {}] Failed to send task: {:?}",
                self.id, e
            );
            return Err(RongJSError::Error(format!(
                "Failed to spawn future on worker {}: channel error: {:?}",
                self.id, e
            )));
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
                Ok(v_any) => {
                    // Downcast happens *here*, just before sending the final result back
                    match v_any.downcast::<R>() {
                        Ok(boxed_r) => Ok(*boxed_r),
                        Err(original_box) => {
                            // Handle the specific case where R is () and the box contains ()
                            if std::any::TypeId::of::<R>() == std::any::TypeId::of::<()>()
                                && original_box.is::<()>()
                            {
                                // SAFETY: We checked R is () and the box contains (), so zeroed is safe.
                                Ok(unsafe { std::mem::zeroed::<R>() })
                            } else {
                                Err(RongJSError::Error(
                                    "Downcast failed in block_on callback".to_string(),
                                ))
                            }
                        }
                    }
                }
                Err(e) => Err(e),
            };
            // Send the final JSResult<R> back to the original caller, ignore error if receiver dropped
            let _ = final_result_tx.send(final_result);
        };

        // Box the callback for sending
        let return_type = UserAsyncReturnType::BlockOn(Box::new(result_callback));

        // Prepare the future_fn that produces Box<dyn Any> (same as before)
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

        // Setup message passing channels for this task
        let (task_message_tx, task_message_rx) = mpsc::channel(100);
        let message_receiver = MessageReceiver::new(task_message_rx);

        // Create task
        let task = UserAsyncTask {
            future_fn: boxed_fn,
            message_receiver,
            task_message_tx,
            return_type,
        };

        // Send task to worker thread (blocking send)
        futures::executor::block_on(async {
            if let Err(e) = self.task_tx.send(task).await {
                return Err(RongJSError::Error(format!(
                    "[block_on Worker {}] Failed to send task: {:?}",
                    self.id, e
                )));
            }
            Ok(())
        })?;

        // Wait for the final JSResult<R> from the callback
        futures::executor::block_on(async {
            final_result_rx.await.map_err(|e| {
                RongJSError::Error(format!(
                    "[block_on Worker {}] Failed to receive final result: {:?}",
                    self.id, e
                ))
            })
        })?
    }

    /// Wait for this worker to complete its current async function
    ///
    /// Returns a future that resolves when the worker's state changes to Free.
    /// This can be awaited to ensure that a worker has finished processing before shutdown.
    pub async fn join(&self) -> JSResult<()> {
        loop {
            // Check state first
            {
                let state_guard = self.state.lock().await;
                if *state_guard == WorkerState::Free {
                    return Ok(());
                }
                // state_guard implicitly dropped here before await
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
        // Send the termination signal by notifying
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
        // Try to send the message, but don't block if the channel is full
        // This is a non-blocking operation that returns immediately
        // The worker loop will receive this and forward if an async function is running
        self.message_tx.try_send(message).map_err(|e| {
            if matches!(e, mpsc::error::TrySendError::Full(_)) {
                eprintln!("Worker {} message channel full, message dropped", self.id);
            } else if matches!(e, mpsc::error::TrySendError::Closed(_)) {
                // This might happen during shutdown
                eprintln!("Worker {} message channel closed, message dropped", self.id);
            }
            // Convert SendError to our error type
            RongJSError::Error(format!(
                "Failed to post message to worker {}: {:?}",
                self.id, e
            ))
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
    /// Number of worker threads
    worker_count: usize,
    /// Size of each worker's task queue
    /// Controls how many pending tasks can be queued before backpressure occurs
    task_queue_size: usize,
    /// Size of each worker's general message queue (for post_message)
    /// Controls how many messages can be buffered before being dropped
    message_queue_size: usize,
    /// Number of net runtime worker threads (>=1)
    net_worker_threads: usize,
    /// Marker for the generic type E
    _marker: PhantomData<E>,
}

impl<E: JSEngine + 'static> RongBuilder<E> {
    /// Create a new builder with default settings
    fn new() -> Self {
        Self {
            worker_count: 4,         // Default to 4 workers instead of num_cpus
            task_queue_size: 100,    // Default task queue size
            message_queue_size: 100, // Default message queue size
            net_worker_threads: 1,   // Default to 1 net worker thread
            _marker: PhantomData,    // Initialize marker
        }
    }

    /// Set the number of worker threads
    pub fn with_num_workers(mut self, count: usize) -> Self {
        if count < 1 {
            panic!("At least one worker thread is required");
        }
        self.worker_count = count;
        self
    }

    /// Set task queue size for each worker
    ///
    /// This controls how many tasks can be queued to a worker before backpressure occurs.
    /// A larger value allows more tasks to be queued without blocking, but consumes more memory.
    /// This is an internal buffer and is generally not exposed to the user.
    pub fn with_task_queue_size(mut self, size: usize) -> Self {
        self.task_queue_size = size;
        self
    }

    /// Set message queue size for each worker
    ///
    /// This controls how many messages can be buffered when sending messages to tasks.
    /// Larger values allow more messages to be buffered without blocking or dropping,
    /// but consume more memory.
    pub fn with_message_queue_size(mut self, size: usize) -> Self {
        self.message_queue_size = size;
        self
    }

    /// Configure the global service runtime worker thread count.
    /// If set, the service runtime will be started during build() with this thread count.
    /// If not set, the service runtime will be lazily started (defaulting to 2 threads)
    /// on first use by callers (e.g., HTTP, background tasks).
    pub fn with_net_threads(mut self, threads: usize) -> Self {
        if threads < 1 {
            panic!("At least one service runtime worker thread is required");
        }
        self.net_worker_threads = threads;
        self
    }

    /// Build and start a Rong instance
    ///
    /// Finalizes the configuration and creates a Rong instance with the specified settings.
    /// After this point, the configuration cannot be changed.
    ///
    /// # Returns
    /// * `Arc<Rong<E>>` - A thread-safe reference to the created Rong instance
    ///
    /// # Example
    /// ```rust
    /// let rong = Rong::builder()
    ///     .with_num_workers(8)
    ///     .with_task_queue_size(200)
    ///     .build();
    /// ```
    pub fn build(self) -> Arc<Rong<E>> {
        // Initialize the shared service runtime with configured threads (idempotent)
        crate::service_executor::start_service_runtime(self.net_worker_threads);

        let rong = Arc::new(Rong {
            workers: Arc::new(TokioMutex::new(Vec::with_capacity(self.worker_count))),
            worker_count: self.worker_count,
            task_queue_size: self.task_queue_size,
            message_queue_size: self.message_queue_size,
        });

        // Initialize the worker pool
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
    /// Worker pool
    workers: Arc<TokioMutex<Vec<Worker<E>>>>,

    /// Number of worker threads
    worker_count: usize,

    /// Size of each worker's task queue
    /// Controls how many pending tasks can be queued before backpressure occurs
    task_queue_size: usize,

    /// Size of each worker's message queue
    /// Controls how many messages can be buffered when sending messages to tasks
    message_queue_size: usize,
}

impl<E: JSEngine + 'static> Rong<E> {
    /// Create a new builder to configure and build a Rong instance
    pub fn builder() -> RongBuilder<E> {
        RongBuilder::new()
    }

    /// Execute a user's async function and wait for the result
    ///
    /// This method automatically gets a free worker and executes the async function on it,
    /// blocking until the function completes and returning its result.
    ///
    /// # Parameters
    /// * `future_fn` - Function that takes a JS runtime and message receiver and returns a future
    ///
    /// # Returns
    /// * `Result<R, RongJSError>` - The result of the async function execution
    ///
    /// # Example
    /// ```rust
    /// let rong = Rong::builder().build();
    /// let result = rong.block_on(|runtime, receiver| async {
    ///     // Your async code here
    ///     Ok(42)
    /// }).unwrap();
    /// ```
    pub fn block_on<F, Fut, R>(&self, future_fn: F) -> JSResult<R>
    where
        F: FnOnce(JSRuntime<E::Runtime>, MessageReceiver) -> Fut + Send + 'static,
        Fut: Future<Output = JSResult<R>> + 'static,
        R: Send + 'static,
    {
        // Get a free worker
        // Need to block here as Rong::block_on is synchronous
        let worker = futures::executor::block_on(self.get_worker())?;

        // Execute the async function on the worker
        worker.block_on(future_fn)
    }

    /// Initialize the worker pool
    ///
    /// Creates and starts all worker threads according to the configured
    /// worker count. Each worker runs in its own thread with a dedicated
    /// task queue and message channel.
    fn initialize_workers(self: &Arc<Self>) {
        // Use Arc<Self> to easily clone for workers
        // Use block_on here as initialize_workers is synchronous
        futures::executor::block_on(async {
            let mut workers_guard = self.workers.lock().await;

            for i in 0..self.worker_count {
                // Create channels for worker communication
                let (task_tx, task_rx) = mpsc::channel(self.task_queue_size);
                let terminate_signal = Arc::new(Notify::new());
                // This channel is for messages sent via post_message
                let (worker_message_tx, worker_message_rx) = mpsc::channel(self.message_queue_size);

                // Create shared state using TokioMutex and Notify
                let state = Arc::new(TokioMutex::new(WorkerState::Free));
                let free_signal = Arc::new(Notify::new());

                // Create worker
                let worker = Worker {
                    id: i,
                    name: None,
                    task_tx: task_tx.clone(),
                    terminate_signal: terminate_signal.clone(),
                    message_tx: worker_message_tx, // Sender for post_message
                    state: state.clone(),
                    free_signal: free_signal.clone(),
                    rong: self.clone(),
                };

                // Add worker to pool
                workers_guard.push(worker);

                // Start worker thread
                let state_clone = state.clone();
                let free_signal_clone = free_signal.clone(); // Clone free signal for thread

                // Pass the worker's message receiver (for post_message) to the thread
                let worker_message_rx_thread: mpsc::Receiver<WorkerMessage> = worker_message_rx;
                // Receiver for tasks is moved into the thread - type is now non-generic UserAsyncTask<E>
                let task_rx_thread: mpsc::Receiver<UserAsyncTask<E>> = task_rx;

                // Spawn a new thread for this worker
                std::thread::spawn(move || {
                    // Create a Tokio runtime for this worker
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .thread_name(format!("worker-{}", i))
                        .build()
                        .expect("Failed to create worker runtime");

                    // Run the worker loop
                    rt.block_on(async {
                        Self::run_worker_loop(
                            i,
                            task_rx_thread, // Pass the worker message receiver
                            worker_message_rx_thread, // Pass the worker message receiver
                            terminate_signal,
                            state_clone,
                            free_signal_clone, // Pass the free signal notifier
                        )
                        .await;
                    });
                });
            }
        });
    }

    /// Run the worker loop
    ///
    /// Core processing loop for a worker thread. This method:
    /// 1. Processes incoming user-provided async functions and executes them with a JavaScript runtime
    /// 2. Handles termination signals
    /// 3. Manages the worker's state based on its current activity
    /// 4. Ensures proper JavaScript microtask execution during async function processing
    /// 5. Forwards messages from post_message to the currently executing async function
    async fn run_worker_loop(
        worker_id: usize,
        mut task_rx: mpsc::Receiver<UserAsyncTask<E>>,
        mut worker_message_rx: mpsc::Receiver<WorkerMessage>,
        terminate_signal: Arc<tokio::sync::Notify>,
        state: Arc<TokioMutex<WorkerState>>,
        free_signal: Arc<Notify>,
    ) {
        // Create a local task executor to ensure all tasks run on this OS thread
        let local = tokio::task::LocalSet::new();

        local
            .run_until(async move {
               let mut should_terminate = false;

                // State for the currently running task
                type TaskJoinHandle = tokio::task::JoinHandle<
                    Result<JSResult<Box<dyn Any + Send>>, futures::future::Aborted>,
                >;
                let mut current_task_join_handle: Option<TaskJoinHandle> = None;
                let mut current_task_abort_handle: Option<futures::future::AbortHandle> = None;
                let mut current_microtask_runner_handle: Option<tokio::task::JoinHandle<()>> = None;
                let mut current_task_message_tx: Option<mpsc::Sender<WorkerMessage>> = None;
                let mut current_js_runtime: Option<JSRuntime<E::Runtime>> = None;
                let mut current_task_result_callback: Option<BlockOnCallback> = None;


                // Main worker event loop
                while !should_terminate {
                    tokio::select! {
                        // Bias select towards checking for termination first
                        biased;

                        // Process termination signal
                        _ = terminate_signal.notified() => {
                            println!("Worker {} received termination signal", worker_id);
                            if let Some(handle) = current_task_abort_handle.take() {
                                println!("Worker {} aborting main task.", worker_id);
                                handle.abort();
                            }
                            if let Some(handle) = current_microtask_runner_handle.take() {
                                println!("Worker {} aborting microtask runner.", worker_id);
                                handle.abort();
                            }
                            should_terminate = true;
                        },

                        // Process new user async functions, only if no task is currently running
                        maybe_task = task_rx.recv(), if current_task_join_handle.is_none() && !should_terminate => {
                            if let Some(user_async_task) = maybe_task {
                                // Set worker state to Busy
                                {
                                    let mut state_guard = state.lock().await;
                                    *state_guard = WorkerState::Busy;
                                }

                                // Create JS Runtime for this task
                                let js_runtime = E::runtime();
                                current_js_runtime = Some(js_runtime.clone()); // Store for microtasks

                                // Store message sender and result callback
                                current_task_message_tx = Some(user_async_task.task_message_tx);
                                // Store the result callback if it's BlockOn
                                match user_async_task.return_type {
                                    UserAsyncReturnType::BlockOn(callback) => {
                                        current_task_result_callback = Some(callback);
                                    }
                                    UserAsyncReturnType::Spawn => {
                                        current_task_result_callback = None;
                                    }
                                }

                                // Prepare the user's future
                                let user_fn = user_async_task.future_fn;
                                let message_receiver = user_async_task.message_receiver;
                                let user_future = user_fn(js_runtime.clone(), message_receiver);

                                // Make task abortable
                                let (abortable_future, abort_handle) = futures::future::abortable(user_future);
                                current_task_abort_handle = Some(abort_handle);

                                // Spawn the user's future onto the LocalSet
                                let task_handle = spawn(abortable_future);
                                current_task_join_handle = Some(task_handle);

                                // Start microtask runner if needed
                                if js_runtime.run_pending_jobs() >= 0 {

                                    let rt_clone = js_runtime.clone(); // Clone for the microtask runner
                                    let microtask_handle = spawn(async move {
                                        let mut interval = tokio::time::interval(std::time::Duration::from_millis(5));
                                        // Loop indefinitely until aborted by the main loop
                                        loop {
                                            interval.tick().await;
                                            rt_clone.run_pending_jobs();
                                        }
                                    });
                                    current_microtask_runner_handle = Some(microtask_handle);
                                }

                            } else {
                                // task_rx closed - terminate the worker loop
                                println!("Worker {} task channel closed.", worker_id);
                                should_terminate = true;
                            }
                        },

                        // Process messages for the currently running task
                        maybe_message = worker_message_rx.recv(), if current_task_message_tx.is_some() => {
                             if let Some(message) = maybe_message {
                                // Forward the message to the current task, ignoring errors (task might have ended)
                                if let Some(tx) = &current_task_message_tx {
                                    if let Err(e) = tx.try_send(message) {
                                        // Log only if the channel wasn't closed (task ended normally)
                                        if matches!(e, mpsc::error::TrySendError::Full(_)) {
                                            eprintln!("Worker {} task message channel full, dropping message: {}", worker_id, e);
                                        }
                                        // Don't log Closed errors, as the task might have just finished
                                    } else {
                                        // If send succeeded, run pending jobs as message might queue JS work
                                        if let Some(rt) = &current_js_runtime {
                                            rt.run_pending_jobs();
                                        }
                                    }
                                }
                             } else {
                                 // worker_message_rx closed, might indicate an issue or shutdown
                                 println!("Worker {} message channel closed.", worker_id);
                                 // Don't terminate immediately, let running task finish or termination signal handle it
                             }
                        },

                        // Wait for the current *user task* (returning dyn Any) to complete
                        maybe_result = async { current_task_join_handle.as_mut().unwrap().await }, if current_task_join_handle.is_some() => {
                            // The user future returns Result<Box<dyn Any>, Aborted> wrapped in Result<_, JoinError>
                            let final_result: JSResult<Box<dyn Any + Send>> = match maybe_result {
                                Ok(Ok(inner_result)) => inner_result, // Task finished successfully (Ok from JoinHandle, Ok from AbortableFuture)
                                Ok(Err(_aborted)) => Err(RongJSError::Error("Task aborted".to_string())), // Ok from JoinHandle, Err from AbortableFuture (aborted)
                                Err(join_error) => { // Err from JoinHandle (task panicked or runtime dropped)
                                     eprintln!("[Worker {}] User task panicked or runtime dropped: {:?}", worker_id, join_error);
                                     Err(RongJSError::Error(format!("User task panicked or runtime dropped: {}", join_error)))
                                }
                            };

                            // Execute the result callback if it exists, passing the Box<dyn Any> result
                            if let Some(callback) = current_task_result_callback.take() {
                                 callback(final_result);
                            } else {
                                 // Spawn task, just log errors maybe
                                 if let Err(e) = final_result {
                                     eprintln!("[Worker {}] Spawned task failed: {:?}", worker_id, e);
                                 }
                            }

                            // Clean up task state (regardless of runner result)
                            current_task_join_handle = None;
                            current_task_abort_handle = None;
                            current_task_message_tx = None;
                            current_js_runtime = None;
                            // current_task_result_callback already taken

                            // Stop and cleanup microtask runner if it exists
                            if let Some(handle) = current_microtask_runner_handle.take() {
                                handle.abort();
                            }

                            // Set worker state back to Free
                            {
                                let mut state_guard = state.lock().await;
                                *state_guard = WorkerState::Free;
                                free_signal.notify_waiters();
                            }
                        },
                    }
                }

                // Final cleanup if terminated while task was running
                if let Some(handle) = current_task_abort_handle.take() {
                     handle.abort();
                }
                if let Some(handle) = current_microtask_runner_handle.take() {
                     handle.abort();
                }

                // Ensure other state is cleared (Handles already aborted/taken above)
                let _ = current_task_result_callback.take();

                // Set worker state back to Free on final exit (safety net)
                {
                    let mut state_guard = state.lock().await;
                    *state_guard = WorkerState::Free;
                    free_signal.notify_waiters();
                }
            })
            .await;
    }

    /// Asynchronously gets a free worker from the pool.
    ///
    /// This method safely finds an available worker, marks it as busy, and returns it,
    /// ensuring exclusive access during allocation.
    /// If no free worker is available, the returned future resolves to an error.
    pub async fn get_worker(&self) -> JSResult<Worker<E>> {
        let workers_guard = self.workers.lock().await;

        // Find a free worker and immediately mark it as busy
        for worker in workers_guard.iter() {
            let mut state_guard = worker.state.lock().await;

            if *state_guard == WorkerState::Free {
                // Mark as busy immediately to prevent race conditions
                *state_guard = WorkerState::Busy;
                // Drop the state guard before returning the worker clone
                drop(state_guard);

                return Ok(worker.clone());
            }
        }

        Err(RongJSError::Error("No free worker available".to_string()))
    }

    /// Get the count of free workers in the pool
    ///
    /// Returns the number of workers currently in the Free state.
    /// This can be used to monitor pool availability.
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
    ///
    /// Returns the total number of workers, regardless of their state.
    pub async fn total_workers_count(&self) -> usize {
        let workers = self.workers.lock().await;
        workers.len()
    }

    /// Wait for all workers to become free
    ///
    /// Returns a future that resolves when all workers in the pool have completed
    /// their current tasks and returned to the Free state.
    pub async fn join_all(&self) -> JSResult<()> {
        let workers_guard = self.workers.lock().await;

        // Clone workers needed for the async block (Arc clones are cheap)
        let workers_to_join = workers_guard.iter().cloned().collect::<Vec<_>>();
        // Drop the guard *before* creating/awaiting futures
        drop(workers_guard);

        // Collect the async join futures from each worker
        let join_futures = workers_to_join.iter().map(|w| w.join());

        // Wait for all async join operations to complete
        match futures::future::try_join_all(join_futures).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Shutdown all workers
    ///
    /// This sends termination signals to all workers, regardless of their state.
    /// Any async functions currently running on workers will be gracefully interrupted.
    /// After calling this method, the worker pool should not be used anymore.
    fn shutdown(&self) -> JSResult<()> {
        // Use block_on since shutdown is called from Drop (synchronous context)
        futures::executor::block_on(async {
            let workers = self.workers.lock().await;
            // Send terminate signal to all workers
            for worker in workers.iter() {
                if let Err(e) = worker.terminate() {
                    eprintln!("Error while terminating worker {}: {:?}", worker.id, e);
                }
            }
        });
        Ok(())
    }
}

impl<E: JSEngine + 'static> Drop for Rong<E> {
    fn drop(&mut self) {
        // Ensure workers are terminated when Rong is dropped by calling the shutdown logic
        let _ = self.shutdown();
        // Stop global net runtime if running
        crate::service_executor::stop_service_runtime();
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

// Manual implementation because derive Clone fails due to E not being Clone bound
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

impl<E: JSEngine + 'static> Drop for Worker<E> {
    fn drop(&mut self) {
        // Signal termination when worker is dropped
        // This ensures termination even if dropped without explicit terminate() call
        self.terminate_signal.notify_waiters();

        // We don't actually need to do anything with the channels - they'll be
        // dropped automatically when this Worker instance is dropped
    }
}
