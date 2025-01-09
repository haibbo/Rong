use crate::function::RustFunc;
use crate::scheduler::Scheduler;
use crate::{JSCodeRunner, JSContext, JSContextImpl, JSObjectOps, JSResult, JSValueImpl};
use std::future::Future;
use std::rc::Rc;

pub trait JSRuntimeImpl {
    /// the JS engine specific type of JavaScript Runtime
    type FfiRuntime: Copy;

    /// The JavaScript context type associated with this runtime
    type Context: JSContextImpl<Runtime = Self>;

    /// Creates JavaScript runtime.
    fn new() -> Self;

    /// Converts the runtime to its FFI (Foreign Function Interface) representation.
    fn to_ffi(&self) -> Self::FfiRuntime;

    /// Runs all pending jobs in the JavaScript runtime.
    /// This includes executing any queued promise callbacks, microtasks, and other pending operations.
    fn run_pending_jobs(&self);
}

pub struct JSRuntime<R: JSRuntimeImpl> {
    inner: Rc<R>,
    scheduler: Rc<Scheduler<R>>,
}

impl<R: JSRuntimeImpl + 'static> JSRuntime<R> {
    pub fn block_on<F, T>(self, future: F) -> JSResult<T>
    where
        F: Future<Output = JSResult<T>> + 'static,
        T: 'static,
    {
        self.scheduler.block_on(future)
    }

    /// # Warning
    /// testing purposes only and don't use it in production code.
    #[doc(hidden)]
    pub fn run_pending_jobs(&self) {
        self.inner.run_pending_jobs();
    }
}

impl<C: JSContextImpl> JSContext<C> {
    /// used to create object instance as function
    pub(crate) fn register_rustfunc_class(&self)
    where
        C: JSCodeRunner,
        C::Value: JSObjectOps + 'static,
    {
        self.register_class::<RustFunc<C::Value>>();
    }
}

pub trait JSEngine: Sized {
    type Value: JSValueImpl + JSObjectOps;
    type Context: JSContextImpl<Value = Self::Value> + JSCodeRunner;
    type Runtime: JSRuntimeImpl<Context = Self::Context> + 'static;

    /// # Warning
    ///
    /// JS engine is responsible for implementing
    #[doc(hidden)]
    fn _runtime() -> Self::Runtime;

    /// # Warning
    ///
    /// JS engine is responsible for implementing
    #[doc(hidden)]
    fn _context(rt: &Self::Runtime) -> Self::Context;

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
        let runtime = Rc::new(Self::_runtime());
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
        let ctx = JSContext::from(Self::_context(&rt.inner));
        ctx.register_rustfunc_class();
        ctx
    }
}
