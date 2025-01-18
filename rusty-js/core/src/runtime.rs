use crate::function::RustFunc;
use crate::scheduler::Scheduler;
use crate::{JSContext, JSContextImpl, JSObjectOps, JSResult, JSValueImpl};
use std::future::Future;
use std::rc::Rc;

pub trait JSRuntimeImpl {
    /// The raw runtime handle type associated with this runtime.
    type RawRuntime;

    /// The JavaScript context type associated with this runtime
    type Context: JSContextImpl;

    /// Creates JavaScript runtime.
    fn new() -> Self;

    /// Converts the runtime to its raw underlying representation.
    ///
    /// # Key Notes
    /// - This provides access to the low-level, engine-specific runtime handle.
    /// - Useful for advanced use cases requiring direct interaction with the engine.
    /// - The returned value is a copy of the raw runtime handle.
    fn to_raw(&self) -> Self::RawRuntime;

    /// Runs all pending jobs in the JavaScript runtime.
    /// This includes executing any queued promise callbacks, microtasks, and other pending operations.
    fn run_pending_jobs(&self);

    /// Runs garbage collection on the JavaScript runtime.
    ///
    /// # Key Notes
    /// - This method triggers a garbage collection cycle to reclaim unused memory.
    /// - The exact behavior depends on the underlying JavaScript engine implementation.
    /// - Use this judiciously as it may impact performance.
    fn run_gc(&self);
}

pub struct JSRuntime<R: JSRuntimeImpl> {
    pub(crate) inner: Rc<R>,
    scheduler: Rc<Scheduler<R>>,
}

impl<R: JSRuntimeImpl> Clone for JSRuntime<R> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            scheduler: self.scheduler.clone(),
        }
    }
}

impl<R: JSRuntimeImpl + 'static> JSRuntime<R> {
    pub fn block_on<F, T>(&self, future: F) -> JSResult<T>
    where
        F: Future<Output = JSResult<T>> + 'static,
        T: 'static,
    {
        self.scheduler.block_on(future)
    }

    pub(crate) fn scheduler(&self) -> &Rc<Scheduler<R>> {
        &self.scheduler
    }

    /// # Warning
    /// testing purposes only and don't use it in production code.
    #[doc(hidden)]
    pub fn run_pending_jobs(&self) {
        self.inner.run_pending_jobs();
    }

    /// Runs garbage collection on the JavaScript runtime.
    ///
    /// # Key Notes
    /// - This method triggers a garbage collection cycle to reclaim unused memory.
    /// - The exact behavior depends on the underlying JavaScript engine implementation.
    /// - Use this judiciously as it may impact performance.
    pub fn run_gc(&self) {
        self.inner.run_gc();
    }
}

impl<C: JSContextImpl> JSContext<C> {
    /// used to create object instance as function
    pub(crate) fn register_rustfunc_class(&self)
    where
        C::Value: JSObjectOps + 'static,
    {
        self.register_class::<RustFunc<C::Value>>();
    }
}

pub trait JSEngine: Sized {
    type Value: JSValueImpl<Context = Self::Context> + JSObjectOps;
    type Context: JSContextImpl<Value = Self::Value, Runtime = Self::Runtime>;
    type Runtime: JSRuntimeImpl<Context = Self::Context> + 'static;

    /// JS engine name
    fn name() -> &'static str;

    /// JS engine version
    fn version() -> String;

    /// Creates a new JavaScript runtime instance.
    ///
    /// # Key Notes
    /// - One thread have only one runtime instance.
    /// - Each runtime has its own scheduler for managing asynchronous tasks.
    /// - This ensures proper isolation and thread-safety in JavaScript execution.
    ///
    /// # Returns
    /// A new `JSRuntime` instance with its associated scheduler.
    fn runtime() -> JSRuntime<Self::Runtime> {
        let runtime = Rc::new(Self::Runtime::new());
        let scheduler = Scheduler::new(runtime.clone());
        JSRuntime {
            inner: runtime,
            scheduler,
        }
    }

    /// Creates a new JavaScript context instance associated with the given runtime.
    ///
    /// # Key Notes
    /// - Each context is tied to a specific runtime instance.
    /// - Automatically registers the Rust function class for interop capabilities.
    /// - Contexts are isolated execution environments within the same runtime.
    ///
    /// # Example
    /// ```rust
    /// let runtime = JSEngine::runtime();
    /// let context = JSEngine::context(&runtime);
    /// ```
    fn context(rt: &JSRuntime<Self::Runtime>) -> JSContext<Self::Context>
    where
        Self::Value: 'static,
    {
        let ctx = JSContext::<Self::Context>::new(rt);
        ctx.register_rustfunc_class();
        ctx
    }
}
