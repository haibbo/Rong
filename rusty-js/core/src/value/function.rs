use crate::parameter::FromParams;
use crate::{
    Class, FromJSValue, IntoJSCallable, IntoJSValue, JSContext, JSContextImpl, JSExceptionHandler,
    JSObject, JSObjectOps, JSValueImpl, RustFunc,
};
use std::ops::Deref;

pub struct JSFunc<V: JSValueImpl>(JSObject<V>);

impl<V: JSValueImpl> Deref for JSFunc<V> {
    type Target = JSObject<V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> IntoJSValue<V> for JSFunc<V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, ctx: &V::Context) -> V {
        self.0.into_js_value(ctx)
    }
}

impl<V: JSObjectOps> JSFunc<V> {
    pub fn name(self, name: &str) -> Self {
        self.0.set("name", name);
        self
    }

    pub fn into_inner(self) -> V {
        self.0.into_inner()
    }
}

impl<C: JSContextImpl> JSContext<C>
where
    C::Value: JSObjectOps + 'static,
    C: JSExceptionHandler,
{
    pub fn register_function<F, P>(&self, f: F) -> JSFunc<C::Value>
    where
        F: IntoJSCallable<C::Value, P> + 'static,
        P: FromParams<C::Value>,
    {
        let func = RustFunc::new(f);
        let length = func.parameter_required_count();
        let value = Class::get::<RustFunc<C::Value>>(&self.inner)
            .map(|class| class.instance::<RustFunc<C::Value>>(func));
        let obj = JSObject::from_js_value(&self.inner, value.unwrap()).unwrap();
        obj.set("length", length);
        JSFunc(obj)
    }
}
