use crate::{FromJSValue, JSObject, JSObjectOps, JSRuntime, JSRuntimeImpl, JSValue, JSValueImpl};
use std::ops::Deref;

pub trait JSContextImpl: Clone {
    type RawContext: Copy;
    type Runtime: JSRuntimeImpl;

    fn new(runtime: &Self::Runtime) -> Self
    where
        Self: Sized;
    fn as_raw(&self) -> &Self::RawContext;
    fn from_ffi(raw: Self::RawContext) -> Self;
}

pub trait JSRawContext {
    type RawContext;
}

pub struct JSContext<C: JSContextImpl> {
    pub(crate) inner: C,
}

impl<C: JSContextImpl> Deref for JSContext<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<C: JSContextImpl> JSContext<C> {
    pub fn new(runtime: &JSRuntime<C::Runtime>) -> Self {
        Self {
            inner: C::new(&runtime.inner),
        }
    }
}

pub trait JSCodeRunner: JSContextImpl {
    type Value: JSValueImpl<Context = Self>;

    /// eval javascript
    fn eval(&self, source: impl AsRef<str>) -> Self::Value;

    /// get global object
    fn global_object(&self) -> Self::Value;
}

impl<'ctx, C> JSContext<C>
where
    C: JSCodeRunner,
{
    /// eval javascript
    pub fn eval<'a, T>(&'a self, source: impl AsRef<str>) -> Result<T, String>
    where
        C::Value: JSObjectOps<'a>,
        T: FromJSValue<'a, C::Value>,
    {
        let raw = self.inner.eval(source);
        let result = JSValue::new(self, raw);

        if let Some(ex) = result.is_exception() {
            Err(ex.into_error().to_string())
        } else {
            T::from_js_value(result)
        }
    }

    /// get global object
    pub fn global_object(&'ctx self) -> JSObject<'ctx, C::Value> {
        let raw = self.inner.global_object();
        JSValue::new(self, raw).into()
    }
}
