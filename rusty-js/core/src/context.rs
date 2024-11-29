use crate::{JSRuntime, JSRuntimeKind, JSTypeOf, JSValue, JSValueError, JSValueInto, JSValueKind};
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

    fn throw_syntax_error(&self, message: impl AsRef<str>) -> Self::Value;
    fn throw_type_error(&self, message: impl AsRef<str>) -> Self::Value;
    fn throw_reference_error(&self, message: impl AsRef<str>) -> Self::Value;
    fn throw_range_error(&self, message: impl AsRef<str>) -> Self::Value;
    fn throw_error(&self, message: impl AsRef<str>) -> Self::Value;

    fn get_last_exception(&self) -> Self::Value;

    // todo: add global
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

impl<C> JSContext<C>
where
    C: JSContextKind + JSCodeRunner,
    C::Value: JSValueKind,
{
    pub fn throw_syntax_error(&self, message: impl AsRef<str>) -> JSValue<C::Value> {
        let raw = self.inner.throw_syntax_error(message);
        JSValue::new(self, raw)
    }

    pub fn throw_type_error(&self, message: impl AsRef<str>) -> JSValue<C::Value> {
        let raw = self.inner.throw_type_error(message);
        JSValue::new(self, raw)
    }

    pub fn throw_reference_error(&self, message: impl AsRef<str>) -> JSValue<C::Value> {
        let raw = self.inner.throw_reference_error(message);
        JSValue::new(self, raw)
    }

    pub fn throw_range_error(&self, message: impl AsRef<str>) -> JSValue<C::Value> {
        let raw = self.inner.throw_range_error(message);
        JSValue::new(self, raw)
    }

    pub fn throw_error(&self, message: impl AsRef<str>) -> JSValue<C::Value> {
        let raw = self.inner.throw_error(message);
        JSValue::new(self, raw)
    }

    pub fn get_last_exception(&self) -> JSValue<C::Value> {
        JSValue::new(self, self.inner.get_last_exception())
    }
}
