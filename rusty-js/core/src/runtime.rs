use crate::function::RustFunc;
use crate::{JSCodeRunner, JSContext, JSContextImpl, JSObjectOps, JSValueImpl};

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
    pub(crate) inner: R,
}

impl<R: JSRuntimeImpl> JSRuntime<R> {
    pub fn run_pending_jobs(&self) {
        self.inner.run_pending_jobs()
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
    type Runtime: JSRuntimeImpl<Context = Self::Context>;

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

    fn runtime() -> JSRuntime<Self::Runtime> {
        JSRuntime {
            inner: Self::_runtime(),
        }
    }

    fn context(rt: &JSRuntime<Self::Runtime>) -> JSContext<Self::Context>
    where
        Self::Value: 'static,
    {
        let ctx = JSContext::from(Self::_context(&rt.inner));
        ctx.register_rustfunc_class();
        ctx
    }
}
