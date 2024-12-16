use crate::{IntoJSValue, JSObject, JSValueImpl};
use std::ops::Deref;

pub struct JSFunc<'ctx, V: JSValueImpl>(JSObject<'ctx, V>);

impl<'ctx, V: JSValueImpl> Deref for JSFunc<'ctx, V> {
    type Target = JSObject<'ctx, V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> JSFunc<'_, V>
where
    V: JSValueImpl,
{
    pub(crate) fn into_inner(self) -> V {
        self.0.into_inner()
    }
}

impl<V> IntoJSValue<V> for JSFunc<'_, V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, ctx: &V::Context) -> V {
        self.0.into_js_value(ctx)
    }
}
