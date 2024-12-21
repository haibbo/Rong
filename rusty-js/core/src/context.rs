use crate::{
    FromJSValue, JSClass, JSObject, JSObjectOps, JSRuntime, JSRuntimeImpl, JSValue, JSValueImpl,
};
use std::ops::Deref;

pub trait JSContextImpl: Clone {
    /// the JS engine specific type of JavaScript Context
    type FfiContext: Copy;
    type Runtime: JSRuntimeImpl;

    fn new(runtime: &Self::Runtime) -> Self
    where
        Self: Sized;

    /// FfiContext implements Copy
    fn to_ffi(&self) -> Self::FfiContext;

    /// the implementation need to make sure it has the ownship, like as new method
    /// generally, it should increase referen count of FFI Context
    fn from_ffi(ctx: Self::FfiContext) -> Self;
}

pub trait JSFfiContext {
    type FfiContext;
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

    /// register class for rust type
    fn register_class<JC>(&self) -> Self::Value
    where
        JC: JSClass<Self::Value>;
}

impl<C> JSContext<C>
where
    C: JSCodeRunner,
{
    /// eval javascript
    pub fn eval<T>(&self, source: impl AsRef<str>) -> Result<T, String>
    where
        C::Value: JSObjectOps,
        T: FromJSValue<C::Value>,
    {
        let raw = self.inner.eval(source);
        let result = JSValue::new(self, raw);

        if let Some(ex) = result.is_exception() {
            Err(ex.into_error().to_string())
        } else {
            T::from_js_value(&self.inner, result.into_inner())
        }
    }

    /// get global object
    pub fn global_object(&self) -> JSObject<C::Value> {
        let raw = self.inner.global_object();
        JSValue::new(self, raw).into()
    }

    pub fn register_class<JC>(&self)
    where
        JC: JSClass<C::Value>,
        C::Value: JSObjectOps,
    {
        let obj = self.global_object();
        let constrcutor = self.inner.register_class::<JC>();
        let constrcutor = JSValue::new(self, constrcutor);
        obj.set(JC::NAME, constrcutor);
    }
}
