use crate::{JSObject, JSObjectOps, JSValueImpl};
use std::ops::Deref;

pub struct JSFunc<'ctx, V: JSValueImpl>(JSObject<'ctx, V>);

impl<'ctx, V: JSValueImpl> Deref for JSFunc<'ctx, V> {
    type Target = JSObject<'ctx, V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'ctx, V> JSFunc<'ctx, V>
where
    V: JSObjectOps<'ctx>,
{
    pub(crate) fn into_inner(self) -> V {
        self.0.into_inner()
    }
}
