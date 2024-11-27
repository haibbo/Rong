use crate::{JSRuntime, JSRuntimeRaw};

pub trait JSContextRaw {
    type Raw: Copy;
    type Runtime: JSRuntimeRaw;

    fn new(runtime: &JSRuntime<Self::Runtime>) -> Self;

    fn as_raw(&self) -> &Self::Raw;
}

pub struct JSContext<C: JSContextRaw> {
    inner: C,
}

impl<C: JSContextRaw> JSContext<C> {
    pub fn new(runtime: &JSRuntime<C::Runtime>) -> Self {
        Self {
            inner: C::new(runtime),
        }
    }

    pub fn as_raw(&self) -> &C::Raw {
        self.inner.as_raw()
    }

    pub fn get_raw(&self) -> C::Raw {
        *self.as_raw()
    }
}
