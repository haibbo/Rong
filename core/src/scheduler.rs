use crate::{JSContext, JSContextImpl, JSResult, JSRuntimeImpl};
use std::cell::Cell;
use std::cell::RefCell;
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
    is_dropped: Cell<bool>,
    shutdown_signal: Rc<Notify>,
    active_tasks: RefCell<usize>,
    tasks_done: Rc<Notify>,
}

impl<R: JSRuntimeImpl> Drop for Scheduler<R> {
    fn drop(&mut self) {
        // println!(
        //     "Scheduler being dropped, active tasks: {}",
        //     self.active_tasks.borrow()
        // );

        // Mark scheduler as dropped and notify all tasks to shutdown
        self.is_dropped.set(true);
        self.shutdown_signal.notify_waiters();

        // Give tasks a chance to complete gracefully
        self.tokio_rt.block_on(async {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        });

        let local_set = self.local_set.clone();
        let active_tasks = self.active_tasks.clone();

        // If there are still active tasks, force shutdown by dropping local_set
        if *active_tasks.borrow() > 0 {
            println!(
                "Scheduler still has {} active tasks, forcing shutdown",
                active_tasks.borrow()
            );
            drop(local_set);
        }

        // Notify that microtasks are done
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
            is_dropped: Cell::new(false),
            shutdown_signal: Rc::new(Notify::new()),
            active_tasks: RefCell::new(0),
            tasks_done: Rc::new(Notify::new()),
        })
    }

    pub(crate) fn get_shutdown_signal(&self) -> Rc<Notify> {
        self.shutdown_signal.clone()
    }

    pub(crate) fn spawn_local(&self, future: Pin<Box<dyn Future<Output = JSResult<()>>>>) {
        let shutdown_signal = self.shutdown_signal.clone();
        let active_tasks = self.active_tasks.clone();
        let tasks_done = self.tasks_done.clone();

        *active_tasks.borrow_mut() += 1;
        // println!("Task started, active tasks: {}", active_tasks.borrow());

        self.local_set.spawn_local(async move {
            let result = tokio::select! {
                res = future => res,
                _ = shutdown_signal.notified() => {
                    // println!("Task cancelled by scheduler shutdown");
                    Ok(())
                }
            };

            *active_tasks.borrow_mut() -= 1;
            let count = *active_tasks.borrow();
            if count == 0 {
                tasks_done.notify_waiters();
            }

            result
        });
    }

    pub(crate) fn block_on<F, T>(&self, future: F) -> JSResult<T>
    where
        F: Future<Output = JSResult<T>> + 'static,
        T: 'static,
    {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let active_tasks = self.active_tasks.clone();
        let tasks_done = self.tasks_done.clone();
        let runtime = self.runtime.clone();

        let local_set = self.local_set.clone();

        *active_tasks.borrow_mut() += 1;

        local_set.spawn_local(async move {
            let result = future.await;
            let _ = sender.send(result);

            *active_tasks.borrow_mut() -= 1;
            let count = *active_tasks.borrow();

            // println!("Main task completed, active tasks: {}", count);
            if count == 0 {
                tasks_done.notify_waiters();
            }
        });

        // Spawn a task to handle microtasks
        let microtask_done = self.microtask_done.clone();
        let shutdown_signal = self.shutdown_signal.clone();
        local_set.spawn_local(async move {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(std::time::Duration::from_millis(1)) => {
                        if let Some(rt) = runtime.upgrade() {
                            rt.run_pending_jobs();
                        } else {
                            break;
                        }
                    }
                        //Microtask loop cancelled by scheduler shutdown
                    _ = shutdown_signal.notified() => break
                }
            }
            microtask_done.notify_waiters();
        });

        let result = self
            .tokio_rt
            .block_on(async { local_set.run_until(async { receiver.await? }).await });

        // After block_on completes, notify all tasks to shutdown
        self.shutdown_signal.notify_waiters();

        // Wait for all tasks to complete while keeping local_set running
        let local_set = self.local_set.clone();
        self.tokio_rt.block_on(async {
            local_set
                .run_until(async {
                    let mut timeout = tokio::time::interval(std::time::Duration::from_millis(100));
                    loop {
                        tokio::select! {
                            _ = self.tasks_done.notified() => {
                                if *self.active_tasks.borrow() == 0 {
                                    // println!("All tasks completed successfully");
                                    break;
                                }
                            }
                            _ = timeout.tick() => {
                                if *self.active_tasks.borrow() == 0 {
                                        break;
                                }
                                println!("Still waiting for {} tasks to complete", self.active_tasks.borrow());
                            }
                        }
                    }
                })
                .await
        });

        result
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
