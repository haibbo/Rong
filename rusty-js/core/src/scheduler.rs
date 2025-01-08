use crate::{JSResult, JSRuntime, JSRuntimeImpl};
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use tokio::runtime::Builder;
use tokio::task::LocalSet;

thread_local! {
    pub static CURRENT_SCHEDULER: RefCell<Option<Rc<dyn SchedulerHandle>>> = RefCell::new(None);
}

pub trait SchedulerHandle {
    fn spawn_boxed(&self, future: Pin<Box<dyn Future<Output = JSResult<()>>>>);
}

pub struct Scheduler<R: JSRuntimeImpl> {
    runtime: Rc<JSRuntime<R>>,
    tokio_rt: tokio::runtime::Runtime,
    local_set: LocalSet,
}

impl<R: JSRuntimeImpl + 'static> SchedulerHandle for Scheduler<R> {
    fn spawn_boxed(&self, future: Pin<Box<dyn Future<Output = JSResult<()>>>>) {
        self.local_set.spawn_local(future);
    }
}

impl<R: JSRuntimeImpl + 'static> Scheduler<R> {
    pub fn new(runtime: JSRuntime<R>) -> Rc<Self> {
        // single thread tokio runtime
        let tokio_rt = Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        let local_set = LocalSet::new();

        let scheduler = Rc::new(Self {
            runtime: Rc::new(runtime),
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
    pub fn block_on<F, T>(&self, future: F) -> JSResult<T>
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
                tokio::task::yield_now().await;
            }
        };
        self.local_set.spawn_local(js_micro_tasks);

        // Run the local set until we get the result
        let result = self.tokio_rt.block_on(async {
            self.local_set
                .run_until(async { receiver.await.expect("Failed to receive result") })
                .await
        });

        // Clean up the current scheduler before returning the result
        Self::clear_current_scheduler();
        result
    }
}
