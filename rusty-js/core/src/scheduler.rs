use crate::{JSContext, JSContextImpl, JSResult, JSRuntimeImpl};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use tokio::runtime::Builder;
use tokio::task::LocalSet;

thread_local! {
    static CURRENT_SCHEDULER: RefCell<Option<Rc<dyn SchedulerHandle>>> = RefCell::new(None);
}

trait SchedulerHandle {
    fn spawn_boxed(&self, future: Pin<Box<dyn Future<Output = JSResult<()>>>>);
}

pub(crate) struct Scheduler<R: JSRuntimeImpl> {
    runtime: Rc<R>,
    tokio_rt: tokio::runtime::Runtime,
    local_set: LocalSet,
}

impl<R: JSRuntimeImpl + 'static> SchedulerHandle for Scheduler<R> {
    fn spawn_boxed(&self, future: Pin<Box<dyn Future<Output = JSResult<()>>>>) {
        self.local_set.spawn_local(future);
    }
}

impl<R: JSRuntimeImpl + 'static> Scheduler<R> {
    pub(crate) fn new(runtime: Rc<R>) -> Rc<Self> {
        // single thread tokio runtime
        let tokio_rt = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        let local_set = LocalSet::new();

        let scheduler = Rc::new(Self {
            runtime,
            tokio_rt,
            local_set,
        });

        Self::set_current_scheduler(scheduler.clone());

        scheduler
    }

    fn set_current_scheduler(scheduler: Rc<Self>) {
        CURRENT_SCHEDULER.with(|current| {
            *current.borrow_mut() = Some(scheduler);
        });
    }

    fn clear_current_scheduler() {
        CURRENT_SCHEDULER.with(|current| {
            *current.borrow_mut() = None;
        });
    }
    pub(crate) fn block_on<F, T>(&self, future: F) -> JSResult<T>
    where
        F: Future<Output = JSResult<T>> + 'static,
        T: 'static,
    {
        // Create a channel to get the result
        let (sender, receiver) = tokio::sync::oneshot::channel();

        // Wrap the main future in a task
        self.local_set.spawn_local(async move {
            let result = future.await;
            let _ = sender.send(result);
        });

        // Create a task to handle JavaScript engine pending jobs
        let runtime = self.runtime.clone();
        let js_micro_tasks = async move {
            loop {
                runtime.run_pending_jobs();
                // Sleep for a short duration instead of yielding
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
        };

        // Run the local set until we get the result
        let result = self.tokio_rt.block_on(async {
            self.local_set
                .run_until(async {
                    // Spawn the microtasks
                    let microtask_handle = self.local_set.spawn_local(js_micro_tasks);

                    // Wait for the main future
                    let result = receiver.await?;

                    // Abort the microtask loop
                    microtask_handle.abort();

                    result
                })
                .await
        });

        // Clean up the current scheduler before returning the result
        Self::clear_current_scheduler();
        result
    }
}

impl<C: JSContextImpl> JSContext<C> {
    /// Spawn a future to be executed by the scheduler
    pub fn spawn_local<F>(&self, future: F)
    where
        F: Future<Output = JSResult<()>> + 'static,
    {
        if let Some(scheduler) = CURRENT_SCHEDULER.with(|s| s.borrow().as_ref().map(|s| s.clone()))
        {
            scheduler.spawn_boxed(Box::pin(future));
        }
    }
}
