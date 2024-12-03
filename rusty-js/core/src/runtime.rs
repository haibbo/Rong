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

impl<R: JSRuntimeImpl> JSRuntime<R> {
    pub fn new() -> Self {
        Self { inner: R::new() }
    }

    pub fn as_raw(&self) -> &R::RawRuntime {
        &self.inner.as_raw()
    }
}
