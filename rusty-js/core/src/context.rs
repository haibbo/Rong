use crate::{JSRuntime, JSRuntimeKind, JSTypeOf, JSValue, JSValueInto, JSValueKind};
use anyhow::anyhow;
use std::any::type_name;
use std::default::Default;

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

pub trait JSCodeRunner: JSContextKind {
    type Value: JSValueKind<Context = Self>;

    fn eval(&self, source: impl AsRef<str>) -> Self::Value;

    // todo: add global
}

impl<C: JSContextKind> JSContext<C> {
    pub fn eval<T>(&self, source: impl AsRef<str>) -> anyhow::Result<T>
    where
        T: Default,
        C: JSCodeRunner,
        C::Value: JSValueInto<T> + JSTypeOf,
    {
        let raw = self.inner.eval(source);
        let result = JSValue::new(self, raw);
        if result.is_exception() {
            Err(anyhow!("TODO: handle exception"))
        } else {
            result.into_rust().ok_or_else(|| {
                anyhow!(
                    "Failed to convert JS value into Rust type: {}",
                    type_name::<T>()
                )
            })
        }
    }
}
