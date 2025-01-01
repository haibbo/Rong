use crate::function::RustFunc;
use crate::{JSCodeRunner, JSContext, JSContextImpl, JSObjectOps, JSValueImpl};

pub trait JSRuntimeImpl {
    /// the JS engine specific type of JavaScript Runtime
    type FfiRuntime: Copy;

    type Context: JSContextImpl<Runtime = Self>;

    fn new() -> Self;
    fn to_ffi(&self) -> Self::FfiRuntime;
}

pub struct JSRuntime<R: JSRuntimeImpl> {
    pub(crate) inner: R,
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
    fn version() -> &'static str;

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
