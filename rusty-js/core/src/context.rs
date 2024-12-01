use crate::{JSRuntime, JSRuntimeKind, JSTypeOf, JSValue, JSValueError, JSValueInto, JSValueKind};
use anyhow::anyhow;
use std::any::type_name;
use std::default::Default;

pub trait JSContextKind {
    type RawContext: Copy;
    type Runtime: JSRuntimeKind;

    fn new(runtime: &JSRuntime<Self::Runtime>) -> Self;
    fn as_raw(&self) -> &Self::RawContext;
}

pub struct JSContext<C: JSContextKind> {
    pub(crate) inner: C,
}

impl<C: JSContextKind> JSContext<C> {
    pub fn new(runtime: &JSRuntime<C::Runtime>) -> Self {
        Self {
            inner: C::new(runtime),
        }
    }

    pub fn as_raw(&self) -> &C::RawContext {
        self.inner.as_raw()
    }

    pub fn get_raw(&self) -> C::RawContext {
        *self.as_raw()
    }
}

pub trait JSCodeRunner: JSContextKind {
    type Value: JSValueKind<Context = Self>;

    fn eval(&self, source: impl AsRef<str>) -> Self::Value;
    fn get_last_exception(&self) -> Self::Value;

    // todo: add global_object()->JSObject,  JSValue into JSObject
}

impl<C: JSContextKind> JSContext<C> {
    pub fn eval<T>(&self, source: impl AsRef<str>) -> anyhow::Result<T>
    where
        T: Default,
        C: JSCodeRunner,
        C::Value: JSValueInto<T> + JSTypeOf + JSValueError,
    {
        let raw = self.inner.eval(source);
        let result = JSValue::new(self, raw);
        if result.is_exception() {
            let exception = self.inner.get_last_exception();
            let result = JSValue::new(self, exception);
            Err(anyhow::Error::from(result.into_error()))
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
