use crate::JSContextImpl;

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
