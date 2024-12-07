use crate::{JSObject, JSObjectOps, JSRuntime, JSRuntimeImpl, JSValue, JSValueImpl, JSValueInto};

pub trait JSContextImpl {
    type RawContext: Copy;
    type Runtime: JSRuntimeImpl;

    fn new(runtime: &Self::Runtime) -> Self
    where
        Self: Sized;
    fn as_raw(&self) -> &Self::RawContext;
}

pub trait JSRawContext {
    type RawContext;
}

pub struct JSContext<C: JSContextImpl> {
    pub(crate) inner: C,
}

impl<C: JSContextImpl> JSContext<C> {
    pub fn new(runtime: &JSRuntime<C::Runtime>) -> Self {
        Self {
            inner: C::new(&runtime.inner),
        }
    }

    pub fn as_raw(&self) -> &C::RawContext {
        self.inner.as_raw()
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
        JSValue<'a, C::Value>: JSValueInto<T>,
    {
        let raw = self.inner.eval(source);
        let result = JSValue::new(self, raw);

        if let Some(ex) = result.is_exception() {
            Err(ex.into_error().to_string())
        } else {
            result.js_into()
        }
    }

    /// get global object
    pub fn global_object(&'ctx self) -> JSObject<'ctx, C::Value> {
        let raw = self.inner.global_object();
        JSValue::new(self, raw).into()
    }
}
