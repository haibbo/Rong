use crate::{JSContext, JSContextImpl, JSResult, JSRuntimeImpl};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use tokio::runtime::Builder;
use tokio::sync::Notify;
use tokio::task::LocalSet;

thread_local! {
    static CURRENT_SCHEDULER: RefCell<Option<Rc<dyn SchedulerHandle + 'static>>> = RefCell::new(None);
}

trait SchedulerHandle {
    fn spawn_boxed(&self, future: Pin<Box<dyn Future<Output = JSResult<()>>>>);
    fn microtask_done(&self) -> Rc<Notify>;
}

pub(crate) struct Scheduler<R: JSRuntimeImpl> {
    runtime: Rc<R>,
    tokio_rt: tokio::runtime::Runtime,
    local_set: LocalSet,
    microtask_done: Rc<Notify>,
}

impl<R: JSRuntimeImpl + 'static> SchedulerHandle for Scheduler<R> {
    fn spawn_boxed(&self, future: Pin<Box<dyn Future<Output = JSResult<()>>>>) {
        self.local_set.spawn_local(future);
    }

    fn microtask_done(&self) -> Rc<Notify> {
        self.microtask_done.clone()
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
            microtask_done: Rc::new(Notify::new()),
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
            if let Some(scheduler) = current.borrow_mut().take() {
                // Notify microtasks to stop
                scheduler.microtask_done().notify_waiters();

                drop(scheduler);
            }
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
        let (done_tx, done_rx) = tokio::sync::watch::channel(false);
        let microtask_done = self.microtask_done.clone();

        // Move js_micro_tasks outside to avoid lifetime issues
        let js_micro_tasks = {
            async move {
                while !*done_rx.borrow() {
                    runtime.run_pending_jobs();
                    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                }
                // One final run to clear any remaining jobs
                runtime.run_pending_jobs();

                // Notify that microtasks are done
                println!("I'm done");
                microtask_done.notify_waiters();
            }
        };

        let result = self.tokio_rt.block_on(async {
            self.local_set
                .run_until(async {
                    self.local_set.spawn_local(js_micro_tasks);

                    let result = receiver.await?;

                    let _ = done_tx.send(true);
                    self.microtask_done.notified().await;
                    println!("Got js done");

                    result
                })
                .await
        });

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
