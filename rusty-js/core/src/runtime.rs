use crate::{JSContext, JSContextImpl, JSValueImpl};

pub trait JSRuntimeImpl {
    /// the JS engine specific type of JavaScript Runtime
    type FfiRuntime: Copy;

    type Context: JSContextImpl;

    fn new() -> Self;
    fn to_ffi(&self) -> Self::FfiRuntime;
}

pub struct JSRuntime<R: JSRuntimeImpl> {
    pub(crate) inner: R,
}

impl<R: JSRuntimeImpl> Default for JSRuntime<R> {
    fn default() -> Self {
        Self { inner: R::new() }
    }
}

pub trait JSEngine: Sized {
    type Value: JSValueImpl;
    type Context: JSContextImpl;
    type Runtime: JSRuntimeImpl;

    /// JS engine is responsible for implementing
    fn _runtime() -> Self::Runtime;
    fn _context(rt: &Self::Runtime) -> Self::Context;

    fn runtime() -> JSRuntime<Self::Runtime> {
        JSRuntime {
            inner: Self::_runtime(),
        }
    }

    fn context(rt: &Self::Runtime) -> JSContext<Self::Context> {
        JSContext::from(Self::_context(rt))
    }
}
