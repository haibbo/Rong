use crate::{JSObject, JSRuntime, JSRuntimeImpl, JSTypeOf, JSValue, JSValueError, JSValueImpl};

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

    /// get last exception
    fn get_last_exception(&self) -> Self::Value;

    /// get global object
    fn global_object(&self) -> Self::Value;
}

impl<'ctx, C> JSContext<C>
where
    C: JSContextImpl + JSCodeRunner,
{
    /// eval javascript
    pub fn eval<T>(&self, source: impl AsRef<str>) -> Result<T, String>
    where
        T: Default,
        C::Value: TryInto<T, Error = String> + JSTypeOf + JSValueError,
    {
        let raw = self.inner.eval(source);
        let result = JSValue::new(self, raw);

        result
            .is_exception()
            .map(|_| {
                let exception = self.inner.get_last_exception();
                let result = JSValue::new(self, exception);
                Err(result.into_error().to_string())
            })
            .unwrap_or_else(|| result.try_into())
    }

    /// get global object
    pub fn global_object(&'ctx self) -> JSObject<'ctx, C::Value> {
        let raw = self.inner.global_object();
        JSValue::new(self, raw).into()
    }
}
