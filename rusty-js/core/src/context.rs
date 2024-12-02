use crate::{JSRuntime, JSRuntimeKind, JSTypeOf, JSValue, JSValueError, JSValueKind};

pub trait JSContextKind {
    type RawContext;
    type Runtime: JSRuntimeKind;

    fn new(runtime: &JSRuntime<Self::Runtime>) -> Self
    where
        Self: Sized;
    fn as_raw(&self) -> &Self::RawContext;
}

pub trait JSRawContext {
    type RawContext;
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
}

pub trait JSCodeRunner: JSContextKind {
    type Value: JSValueKind<Context = Self>;

    fn eval(&self, source: impl AsRef<str>) -> Self::Value;
    fn get_last_exception(&self) -> Self::Value;

    // todo: add global_object()->JSObject,  JSValue into JSObject
}

impl<C: JSContextKind> JSContext<C> {
    pub fn eval<T>(&self, source: impl AsRef<str>) -> Result<T, String>
    where
        T: Default,
        C: JSCodeRunner,
        C::Value: TryInto<T, Error = String> + JSTypeOf + JSValueError,
    {
        let raw = self.inner.eval(source);
        let result = JSValue::new(self, raw);
        if result.is_exception() {
            let exception = self.inner.get_last_exception();
            let result = JSValue::new(self, exception);
            Err(result.into_error().to_string())
        } else {
            result.try_into()
        }
    }
}
