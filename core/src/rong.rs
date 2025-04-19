use crate::{JSEngine, JSResult, JSRuntime, RongJSError};
use std::any::Any;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};

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

/// Message receiver for workers to receive posted messages
///
/// Each user task running on a worker receives its own MessageReceiver instance,
/// allowing it to receive messages posted to the worker.
pub struct MessageReceiver {
    /// Channel for receiving messages from the worker's broadcast channel
    receiver: mpsc::Receiver<Box<dyn Any + Send>>,
}

impl MessageReceiver {
    /// Create a new message receiver from a channel
    fn new(receiver: mpsc::Receiver<Box<dyn Any + Send>>) -> Self {
        Self { receiver }
    }

    /// Try to receive a message without blocking
    pub fn try_recv(&mut self) -> Result<Box<dyn Any + Send>, mpsc::error::TryRecvError> {
        self.receiver.try_recv()
    }

    /// Receive a message asynchronously
    pub async fn recv(&mut self) -> Option<Box<dyn Any + Send>> {
        self.receiver.recv().await
    }
}

// Type alias for the boxed future eventually produced by the closure in UserAsyncTask
type BoxedTaskFuture = Pin<Box<dyn Future<Output = JSResult<Box<dyn Any + Send>>>>>;

// Type alias for the boxed closure stored in UserAsyncTask
type BoxedFutureFn<E> =
    Box<dyn FnOnce(JSRuntime<<E as JSEngine>::Runtime>, MessageReceiver) -> BoxedTaskFuture + Send>;

/// Internal representation of a user-submitted asynchronous function submitted to a worker.
/// Holds the necessary components to invoke the user's future on the worker thread.
struct UserAsyncTask<E: JSEngine + 'static> {
    // Store the closure and receiver, not the final future, to avoid !Send issues with JSRuntime
    // The closure produces the boxed Any result type expected by result_tx
    future_fn: BoxedFutureFn<E>,
    message_receiver: MessageReceiver,

    /// Channel to send the final result (or error/abort) back to the caller.
    result_tx: oneshot::Sender<JSResult<Box<dyn Any + Send>>>,

    /// Channel for the worker loop to forward post_message messages to this user's async function.
    task_message_tx: mpsc::Sender<Box<dyn Any + Send>>,
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
    message_tx: mpsc::Sender<Box<dyn Any + Send>>,

    /// Worker state (Free/Busy)
    state: Arc<Mutex<WorkerState>>,

    /// Signal for when the worker becomes free
    free_signal: Arc<tokio::sync::Notify>,

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
    pub fn state(&self) -> WorkerState {
        *self.state.lock().unwrap()
    }

    /// Private helper to create and submit a user's async function to the worker queue.
    /// Returns a receiver for the async function's final result.
    fn submit_async_fn<F, Fut, R>(
        &self,
        future_fn: F,
    ) -> JSResult<oneshot::Receiver<JSResult<Box<dyn Any + Send>>>>
    where
        F: FnOnce(JSRuntime<E::Runtime>, MessageReceiver) -> Fut + Send + 'static,
        Fut: Future<Output = JSResult<R>> + 'static,
        R: Send + 'static,
    {
        // Set up channels for the async function execution
        let (task_message_tx, task_message_rx) = mpsc::channel(100);
        let (result_tx, result_rx) = oneshot::channel();

        // Create worker message receiver
        let message_receiver = MessageReceiver::new(task_message_rx);

        // Wrap user's future_fn to convert its result to Box<dyn Any + Send>
        let boxed_future_fn: BoxedFutureFn<E> = Box::new(move |rt, recv| {
            // Wrap future to box result
            let user_future = future_fn(rt, recv);
            Box::pin(async move {
                user_future
                    .await
                    .map(|result| Box::new(result) as Box<dyn Any + Send>)
            })
        });

        // Create task with message handler
        let task = UserAsyncTask {
            future_fn: boxed_future_fn,
            message_receiver,
            result_tx,
            task_message_tx,
        };

        // Send async function to worker thread
        // Use blocking send to ensure it is actually sent
        futures::executor::block_on(async {
            if let Err(e) = self.task_tx.send(task).await {
                return Err(RongJSError::Error(format!(
                    "Failed to send async function to worker {}: {:?}",
                    self.id, e
                )));
            }
            Ok(result_rx)
        })
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
        // Perform type erasure internally.
        let boxed_fn: BoxedFutureFn<E> = Box::new(
            move |runtime: JSRuntime<E::Runtime>, receiver: MessageReceiver| {
                // 1. Call user's function to get the anonymous Future `Fut`
                let user_fut: Fut = future_fn(runtime, receiver);
                // 2. Box and Pin it *immediately* for type erasure
                let user_fut_boxed = Box::pin(user_fut);
                // 3. Create the mapping future that awaits the boxed future
                //    and converts the result R to Box<dyn Any + Send>
                let mapped_fut = async move {
                    match user_fut_boxed.await {
                        Ok(r) => Ok(Box::new(r) as Box<dyn Any + Send>),
                        Err(e) => Err(e),
                    }
                };
                // 4. Return the pinned, boxed mapping future, cast to BoxedTaskFuture
                Box::pin(mapped_fut) as BoxedTaskFuture
            },
        );

        // Submit the boxed function using the internal helper
        #[allow(clippy::let_underscore_future)]
        let _ = self.submit_async_fn(boxed_fn)?;
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
        // Perform type erasure
        let boxed_fn: BoxedFutureFn<E> = Box::new(
            move |runtime: JSRuntime<E::Runtime>, receiver: MessageReceiver| {
                // 1. Call user's function to get the anonymous Future `Fut`
                let user_fut: Fut = future_fn(runtime, receiver);
                // 2. Box and Pin it *immediately* for type erasure
                let user_fut_boxed = Box::pin(user_fut);
                // 3. Create the mapping future that awaits the boxed future
                //    and converts the result R to Box<dyn Any + Send>
                let mapped_fut = async move {
                    match user_fut_boxed.await {
                        Ok(r) => Ok(Box::new(r) as Box<dyn Any + Send>),
                        Err(e) => Err(e),
                    }
                };
                // 4. Return the pinned, boxed mapping future, cast to BoxedTaskFuture
                Box::pin(mapped_fut) as BoxedTaskFuture
            },
        );

        let result_rx = self.submit_async_fn(boxed_fn)?;

        // Wait for the result
        let result = futures::executor::block_on(async {
            result_rx.await.map_err(|e| {
                RongJSError::Error(format!(
                    "Failed to receive result from worker {}: {:?}",
                    self.id, e
                ))
            })
        })?;

        // Downcast the result
        match result {
            Ok(v_any) => {
                // Special handling for () type
                if std::any::TypeId::of::<R>() == std::any::TypeId::of::<()>() {
                    // Using zeroed memory is safe here because () has no fields
                    let unit_value = unsafe { std::mem::zeroed::<R>() };
                    return Ok(unit_value);
                }

                // For non-() types, perform normal downcast
                Ok(*(v_any.downcast::<R>().map_err(|_| {
                    RongJSError::Error("Failed to downcast result to expected type R".to_string())
                })?))
            }
            Err(e) => Err(e),
        }
    }

    /// Wait for this worker to complete its current async function
    ///
    /// Blocks the calling thread until the worker's state changes to Free.
    /// This can be used to ensure that a worker has finished processing before shutdown.
    pub async fn join(&self) -> JSResult<()> {
        loop {
            // Check state first
            {
                let state = self.state.lock().unwrap();
                if *state == WorkerState::Free {
                    return Ok(());
                }
            } // Lock released before await

            // Wait for notification that state *might* be Free
            self.free_signal.notified().await;

            // Loop will re-check state after notification
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
    pub fn post_message(&self, value: Box<dyn Any + Send>) -> JSResult<()> {
        // Try to send the message, but don't block if the channel is full
        // This is a non-blocking operation that returns immediately
        // The worker loop will receive this and forward if an async function is running
        let _ = self.message_tx.try_send(value).map_err(|e| {
            if matches!(e, mpsc::error::TrySendError::Full(_)) {
                eprintln!("Worker {} message channel full, message dropped", self.id);
            } else if matches!(e, mpsc::error::TrySendError::Closed(_)) {
                // This might happen during shutdown
                eprintln!("Worker {} message channel closed, message dropped", self.id);
            }
            // Convert SendError to our error type, although we are ignoring it with let _
            RongJSError::Error(format!(
                "Failed to post message to worker {}: {:?}",
                self.id, e
            ))
        });
        Ok(())
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
        let rong = Arc::new(Rong {
            workers: Mutex::new(Vec::with_capacity(self.worker_count)),
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
    workers: Mutex<Vec<Worker<E>>>,

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
        let worker = self.get_worker()?;

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
        let mut workers = self.workers.lock().unwrap();

        for i in 0..self.worker_count {
            // Create channels for worker communication
            let (task_tx, task_rx) = mpsc::channel(self.task_queue_size);
            let terminate_signal = Arc::new(tokio::sync::Notify::new());
            // This channel is for messages sent via post_message
            let (worker_message_tx, worker_message_rx) = mpsc::channel(self.message_queue_size);

            // Create shared state
            let state = Arc::new(Mutex::new(WorkerState::Free));
            let free_signal = Arc::new(tokio::sync::Notify::new());

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
            workers.push(worker);

            // Start worker thread
            let state_clone = state.clone();
            let free_signal_clone = free_signal.clone(); // Clone free signal for thread

            // Pass the worker's message receiver (for post_message) to the thread
            let worker_message_rx_thread = worker_message_rx;
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
                        task_rx_thread,           // Pass the worker message receiver
                        worker_message_rx_thread, // Pass the worker message receiver
                        terminate_signal,
                        state_clone,
                        free_signal_clone, // Pass the free signal notifier
                    )
                    .await;
                });
            });
        }
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
        mut worker_message_rx: mpsc::Receiver<Box<dyn Any + Send>>,
        terminate_signal: Arc<tokio::sync::Notify>,
        state: Arc<Mutex<WorkerState>>,
        free_signal: Arc<tokio::sync::Notify>,
    ) {
        // Create a local task executor to ensure all tasks run on this OS thread
        let local = tokio::task::LocalSet::new();

        local
            .run_until(async move {
                // Set worker state to free initially
                {
                    let mut state_guard = state.lock().unwrap();
                    *state_guard = WorkerState::Free;
                    free_signal.notify_waiters();
                }

                let mut should_terminate = false;
                let mut current_task_message_tx: Option<mpsc::Sender<Box<dyn Any + Send>>> = None;
                let mut current_task_abort_handle: Option<futures::future::AbortHandle> = None;

                // Main worker event loop
                while !should_terminate {
                    // Use tokio::select to handle async functions, messages, termination signal and channel closure
                    tokio::select! {
                        // Bias select towards checking for termination first
                        biased;

                        // Process termination signal
                        _ = terminate_signal.notified() => {
                            println!("Worker {} received termination signal", worker_id);
                            // Abort any running task and take the handle to properly drop it
                            let _ = current_task_abort_handle.take().map(|handle| {
                                println!("Worker {} aborting current task.", worker_id);
                                handle.abort();
                            });
                            should_terminate = true;
                        },

                        // Process worker's user async functions
                        maybe_task = task_rx.recv(), if current_task_message_tx.is_none() && !should_terminate => {
                            if let Some(user_async_task) = maybe_task {
                                // Create JS Runtime and execute user's async function
                                let js_runtime = E::runtime();
                                // Clone message_tx for forwarding
                                if current_task_message_tx.replace(user_async_task.task_message_tx.clone()).is_some() {
                                    // This should never happen - we only process async functions when current_task_message_tx is None
                                    eprintln!("Worker {} already had an async function running?", worker_id);
                                }

                                // Execute the user's async function
                                let user_fn = user_async_task.future_fn;
                                let message_receiver = user_async_task.message_receiver;
                                let result_tx = user_async_task.result_tx;

                                // Execute the user function
                                let user_future = user_fn(js_runtime.clone(), message_receiver);

                                // Make task abortable and check
                                let (abortable_future, abort_handle) = futures::future::abortable(user_future);
                                if current_task_abort_handle.replace(abort_handle).is_some() {
                                    // This should never happen - an abort handle already existed
                                    eprintln!("Worker {} already had an abort handle?", worker_id);
                                }

                                // Only create and run the microtask handler if needed
                                let microtask_notifier = if js_runtime.run_pending_jobs()>=0 {

                                    // Create a notifier to signal when the user future completes
                                    let notifier = Arc::new(tokio::sync::Notify::new());
                                    let notifier_clone = notifier.clone();

                                    // Run pending JS jobs in a local task
                                    let js_runtime_clone = js_runtime.clone();
                                    tokio::task::spawn_local(async move {
                                        loop {
                                            // Check if completion notification received
                                            let timeout = tokio::time::timeout(
                                                std::time::Duration::from_millis(5),
                                                notifier_clone.notified()
                                            ).await;

                                            // Run pending JS jobs
                                            js_runtime_clone.run_pending_jobs();

                                            // Exit loop if future completed
                                            if timeout.is_ok() {
                                                break;
                                            }
                                        }
                                    });

                                    Some(notifier)
                                } else {
                                    None
                                };

                                // Wait for user future to complete
                                let result_from_future = abortable_future.await;

                                // Notify the JS job runner that the user future is complete (if it exists)
                                if let Some(notifier) = microtask_notifier {
                                    notifier.notify_one();
                                }

                                // Clear any abort handles and message channels
                                let _old_abort_handle = current_task_abort_handle.take();
                                let _old_message_tx = current_task_message_tx.take();

                                // Handle result
                                let final_result: JSResult<Box<dyn Any + Send>> = match result_from_future {
                                    Ok(inner_result) => inner_result,
                                    Err(_) => Err(RongJSError::Error("Task aborted by worker shutdown".to_string()))
                                };

                                // Send the result back to the caller, ignoring send errors (caller may have dropped)
                                let _ = result_tx.send(final_result);

                                // Set worker state back to Free
                                {
                                    let mut state_guard = state.lock().unwrap();
                                    *state_guard = WorkerState::Free;
                                    free_signal.notify_waiters();
                                }
                            } else {
                                // task_rx closed - terminate the worker loop
                                should_terminate = true;
                            }
                        },

                        // Process messages to the currently running task
                        Some(message) = worker_message_rx.recv(), if current_task_message_tx.is_some() => {
                            // Forward the message to the current task, ignoring errors
                            if let Some(tx) = &current_task_message_tx {
                                // Restore error logging on failed send
                                if let Err(e) = tx.try_send(message) {
                                    eprintln!("Worker {} failed to forward message to task: {}", worker_id, e);
                                }
                            }
                        },

                        // If worker_message_rx closed, we should terminate
                        else => {
                            should_terminate = true;
                        }
                    }
                }

                // Final cleanup - ensure we've dropped any task handles
                let _ = current_task_abort_handle.take();
                let _ = current_task_message_tx.take();

                // Set worker state back to Free on exit
                {
                    let mut state_guard = state.lock().unwrap();
                    *state_guard = WorkerState::Free;
                    free_signal.notify_waiters();
                }
            })
            .await;
    }

    /// Get a free worker from the pool
    ///
    /// This method atomically finds an available worker and marks it as busy
    /// before returning it, ensuring thread-safety in worker allocation.
    /// If no free worker is available, returns an error.
    pub fn get_worker(&self) -> JSResult<Worker<E>> {
        let workers_guard = self.workers.lock().unwrap();

        // Find a free worker and immediately mark it as busy
        for worker in workers_guard.iter() {
            // Get mutex lock on worker state
            let mut state_guard = worker.state.lock().unwrap();

            // Check if worker is free and atomically mark it as busy if it is
            if *state_guard == WorkerState::Free {
                // Mark as busy immediately to prevent race conditions
                *state_guard = WorkerState::Busy;

                // Return the worker (already marked as busy)
                return Ok(worker.clone());
            }
        }

        // No free worker available
        Err(RongJSError::Error("No free worker available".to_string()))
    }

    /// Get the count of free workers in the pool
    ///
    /// Returns the number of workers currently in the Free state.
    /// This can be used to monitor pool availability.
    pub fn free_workers_count(&self) -> usize {
        let workers = self.workers.lock().unwrap();

        workers
            .iter()
            .filter(|w| *w.state.lock().unwrap() == WorkerState::Free)
            .count()
    }

    /// Get total number of workers in the pool
    ///
    /// Returns the total number of workers, regardless of their state.
    pub fn total_workers_count(&self) -> usize {
        let workers = self.workers.lock().unwrap();
        workers.len()
    }

    /// Wait for all workers to become free
    ///
    /// Blocks the calling thread until all workers in the pool have completed
    /// their current tasks and returned to the Free state.
    pub fn join_all(&self) -> JSResult<()> {
        let workers_guard = self.workers.lock().unwrap();
        // Clone workers needed for the async block (Arc clones are cheap)
        let workers_to_join = workers_guard.iter().cloned().collect::<Vec<_>>();
        drop(workers_guard); // Release the lock early (Corrected: drop the guard)

        // Collect the async join futures from each worker
        let join_futures = workers_to_join.iter().map(|w| w.join());

        // Wait for all async join operations to complete by blocking the current thread
        futures::executor::block_on(async {
            match futures::future::try_join_all(join_futures).await {
                Ok(_) => Ok(()),
                // Propagate the first error encountered during join
                Err(e) => Err(e),
            }
        })
    }

    /// Shutdown all workers
    ///
    /// This sends termination signals to all workers, regardless of their state.
    /// Any async functions currently running on workers will be gracefully interrupted.
    /// After calling this method, the worker pool should not be used anymore.
    fn shutdown(&self) -> JSResult<()> {
        let workers = self.workers.lock().unwrap();

        // Send terminate signal to all workers
        for worker in workers.iter() {
            if let Err(e) = worker.terminate() {
                eprintln!("Error while terminating worker {}: {:?}", worker.id, e);
                // Continue with other workers even if one fails
            }
        }

        Ok(())
    }
}

impl<E: JSEngine + 'static> Drop for Rong<E> {
    fn drop(&mut self) {
        // Ensure workers are terminated when Rong is dropped by calling the shutdown logic
        let _ = self.shutdown();
    }
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
