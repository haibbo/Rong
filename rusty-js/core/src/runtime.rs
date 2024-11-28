use crate::JSContextKind;

pub trait JSRuntimeKind {
    // type of raw JS runtime
    type Raw;
    type Context: JSContextKind;

    // new raw JS Runtime
    fn new() -> Self;

    fn as_raw(&self) -> &Self::Raw;
}

pub struct JSRuntime<R: JSRuntimeKind> {
    inner: R,
}

impl<R: JSRuntimeKind> JSRuntime<R> {
    pub fn new() -> Self {
        Self { inner: R::new() }
    }

    pub fn as_raw(&self) -> &R::Raw {
        self.inner.as_raw()
    }
}
