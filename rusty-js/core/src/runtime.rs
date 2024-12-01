use crate::JSContextKind;

pub trait JSRuntimeKind {
    // type of raw JS runtime
    type RawRuntime;
    type Context: JSContextKind;

    fn new() -> Self;
    fn as_raw(&self) -> &Self::RawRuntime;
}

pub struct JSRuntime<R: JSRuntimeKind> {
    inner: R,
}

impl<R: JSRuntimeKind> JSRuntime<R> {
    pub fn new() -> Self {
        Self { inner: R::new() }
    }

    pub fn as_raw(&self) -> &R::RawRuntime {
        self.inner.as_raw()
    }
}
