use crate::JSContextImpl;

pub trait JSRuntimeImpl {
    type RawRuntime: Copy;
    type Context: JSContextImpl;

    fn new() -> Self;
    fn as_raw(&self) -> &Self::RawRuntime;
}

pub struct JSRuntime<R: JSRuntimeImpl> {
    pub(crate) inner: R,
}

impl<R: JSRuntimeImpl> Default for JSRuntime<R> {
    fn default() -> Self {
        Self { inner: R::new() }
    }
}
