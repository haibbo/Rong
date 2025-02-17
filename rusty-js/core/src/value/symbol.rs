use crate::{
    FromJSValue, IntoJSValue, JSContext, JSObject, JSObjectOps, JSResult, JSTypeOf, JSValue,
    JSValueImpl, JSValueMapper, RustyJSError,
};
use std::ops::Deref;

#[derive(PartialEq, Eq, Clone)]
pub struct JSSymbol<V: JSValueImpl>(JSObject<V>);

impl<V> JSSymbol<V>
where
    V: JSObjectOps + JSValueMapper<V>,
{
    /// create JS Symbol Value
    pub fn new(ctx: &JSContext<V::Context>, descripiton: impl AsRef<str>) -> JSResult<Self> {
        let value = V::create_symbol(ctx.as_ref(), descripiton.as_ref());
        value.try_map(|value| Self(JSValue::from_raw(ctx, value).into()))
    }

    pub fn descripiton(&self) -> JSResult<String> {
        self.0.get::<_, String>("description")
    }
}

impl<V: JSValueImpl> Deref for JSSymbol<V> {
    type Target = JSObject<V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> FromJSValue<V> for JSSymbol<V>
where
    V: JSTypeOf,
{
    fn from_js_value(ctx: &JSContext<V::Context>, value: V) -> JSResult<Self> {
        if value.is_symbol() {
            Ok(Self(JSValue::from_raw(ctx, value).into()))
        } else {
            Err(RustyJSError::NotSymbol)
        }
    }
}

impl<V> IntoJSValue<V> for JSSymbol<V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, ctx: &JSContext<V::Context>) -> V {
        self.0.into_js_value(ctx)
    }
}
