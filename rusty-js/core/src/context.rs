use crate::{JSRuntime, JSRuntimeKind};

pub trait JSContextKind {
    type Raw: Copy;
    type Runtime: JSRuntimeKind;

    fn new(runtime: &JSRuntime<Self::Runtime>) -> Self;

    fn as_raw(&self) -> &Self::Raw;
}

pub struct JSContext<C: JSContextKind> {
    inner: C,
}

impl<C: JSContextKind> JSContext<C> {
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
