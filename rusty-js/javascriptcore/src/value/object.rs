use crate::{jsc, JSCValue};
use rusty_js_core::{JSObjectOps, JSValueImpl, PropertyAttributes};

impl JSObjectOps for JSCValue {
    fn new_object(ctx: &Self::Context) -> Self {
        todo!()
    }

    fn make_object<T>(ctx: &Self::Context, constructor: Self, data: *mut T) -> Self {
        todo!()
    }

    fn get_opaque<T>(&self) -> *mut T {
        todo!()
    }

    fn del_property(&self, key: Self) -> bool {
        todo!()
    }

    fn has_property(&self, key: Self) -> bool {
        todo!()
    }

    fn set_property(&self, key: Self, value: Self) -> bool {
        todo!()
    }

    fn set_prototype(&self, prototype: Self) -> bool {
        todo!()
    }

    fn define_property(
        &self,
        key: Self,
        value: Self,
        getter: Self,
        setter: Self,
        attributes: PropertyAttributes,
    ) -> bool {
        todo!()
    }

    fn get_property(&self, key: Self) -> Option<Self> {
        todo!()
    }
}
