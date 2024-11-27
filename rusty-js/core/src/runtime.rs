use crate::JSContextRaw;

pub trait JSRuntimeRaw {
    // type of raw JS runtime
    type Raw;
    type Context: JSContextRaw;

    // new raw JS Runtime
    fn new() -> Self;

    fn as_raw(&self) -> &Self::Raw;
}

pub struct JSRuntime<R: JSRuntimeRaw> {
    inner: R,
}

impl<R: JSRuntimeRaw> JSRuntime<R> {
    pub fn new() -> Self {
        Self { inner: R::new() }
    }

    pub fn as_raw(&self) -> &R::Raw {
        self.inner.as_raw()
    }
}
