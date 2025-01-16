use crate::{JSContext, JSContextImpl, JSResult, JSRuntimeImpl};
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::{Rc, Weak};
use tokio::runtime::Builder;
use tokio::sync::Notify;
use tokio::task::LocalSet;

pub(crate) struct Scheduler<R: JSRuntimeImpl> {
    runtime: Weak<R>,
    tokio_rt: tokio::runtime::Runtime,
    local_set: Rc<LocalSet>,
    microtask_done: Rc<Notify>,
    is_dropped: Rc<Cell<bool>>,
}

impl<R: JSRuntimeImpl> Drop for Scheduler<R> {
    fn drop(&mut self) {
        self.is_dropped.set(true);
        self.microtask_done.notify_waiters();
    }
}

impl<R: JSRuntimeImpl + 'static> Scheduler<R> {
    pub(crate) fn new(runtime: Rc<R>) -> Rc<Self> {
        Rc::new(Self {
            runtime: Rc::downgrade(&runtime),
            tokio_rt: Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime"),
            local_set: Rc::new(LocalSet::new()),
            microtask_done: Rc::new(Notify::new()),
            is_dropped: Rc::new(Cell::new(false)),
        })
    }

    pub(crate) fn spawn_local(&self, future: Pin<Box<dyn Future<Output = JSResult<()>>>>) {
        self.local_set.spawn_local(future);
    }

    pub(crate) fn block_on<F, T>(&self, future: F) -> JSResult<T>
    where
        F: Future<Output = JSResult<T>> + 'static,
        T: 'static,
    {
        let (sender, receiver) = tokio::sync::oneshot::channel();

        let local_set = self.local_set.clone();
        local_set.spawn_local(async move {
            let result = future.await;
            let _ = sender.send(result);
        });

        self.runtime.upgrade().expect("Failed to upgrade runtime");
        let (done_tx, done_rx) = tokio::sync::watch::channel(false);
        let microtask_done = self.microtask_done.clone();

        let js_micro_tasks = {
            let is_dropped = self.is_dropped.clone();
            let runtime_weak = self.runtime.clone();
            async move {
                while !*done_rx.borrow() && !is_dropped.get() {
                    if let Some(rt) = runtime_weak.upgrade() {
                        rt.run_pending_jobs();
                    } else {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                }
                microtask_done.notify_waiters();
            }
        };

        self.tokio_rt.block_on(async {
            local_set
                .run_until(async {
                    local_set.spawn_local(js_micro_tasks);

                    let result = receiver.await?;

                    let _ = done_tx.send(true);
                    self.microtask_done.notified().await;

                    result
                })
                .await
        })
    }
}

impl<C: JSContextImpl> JSContext<C>
where
    C::Runtime: 'static,
{
    pub fn spawn_local<F>(&self, future: F)
    where
        F: Future<Output = JSResult<()>> + 'static,
    {
        let scheduler = self.runtime().scheduler();
        scheduler.spawn_local(Box::pin(future));
    }
}
