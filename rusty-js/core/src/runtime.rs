use crate::JSContextKind;

pub trait JSRuntimeKind {
    type RawRuntime: Copy;
    type Context: JSContextKind;

    fn new() -> Self;
    fn as_raw(&self) -> &Self::RawRuntime;
}

pub struct JSRuntime<R: JSRuntimeKind> {
    pub(crate) inner: R,
}

impl<R: JSRuntimeKind> JSRuntime<R> {
    pub fn new() -> Self {
        Self { inner: R::new() }
    }

    pub fn as_raw(&self) -> &R::RawRuntime {
        &self.inner.as_raw()
    }
}
