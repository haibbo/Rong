use crate::qjs;
use crate::QJSValue;
use rusty_js_core::{JSContextImpl, JSObjectOps, JSValueImpl, PropertyAttributes};

impl<'ctx> JSObjectOps<'ctx> for QJSValue {
    fn new_object(ctx: &'ctx Self::Context) -> Self {
        let ctx = *ctx.as_raw();
        let v = unsafe { qjs::JS_NewObject(ctx) };
        QJSValue::from_ffi(ctx, v)
    }

    fn make_object<T>(ctx: &'ctx Self::Context, constructor: Self, data: *mut T) -> Self {
        let ctx = *ctx.as_raw();
        let v = unsafe { qjs::QJS_ObjectMake(ctx, constructor.value, data.cast()) };
        QJSValue::from_ffi(ctx, v)
    }

    fn get_opaque<T>(&self) -> *mut T {
        let v = unsafe { qjs::QJS_ObjectGetPrivate(self.value) };
        v as *mut T
    }

    fn del_property(&self, key: Self) -> bool {
        let v = unsafe {
            let atom = qjs::JS_ValueToAtom(self.ctx, key.value);
            let v = qjs::JS_DeleteProperty(
                self.ctx, self.value, atom, 0, // flags is 0, OK ?
            );
            qjs::JS_FreeAtom(self.ctx, atom);
            v
        };

        v != 0
    }

    fn has_property(&self, key: Self) -> bool {
        let v = unsafe {
            let atom = qjs::JS_ValueToAtom(self.ctx, key.value);
            let v = qjs::JS_HasProperty(self.ctx, self.value, atom);
            qjs::JS_FreeAtom(self.ctx, atom);
            v
        };

        v != 0
    }

    fn set_property(&self, key: Self, value: Self) -> bool {
        let kv = value.into_raw_value();
        let v = unsafe {
            let atom = qjs::JS_ValueToAtom(self.ctx, key.value);
            let v = qjs::JS_SetProperty(self.ctx, self.value, atom, kv);
            qjs::JS_FreeAtom(self.ctx, atom);
            v
        };
        v != 0
    }

    fn get_property(&self, key: Self) -> Option<Self> {
        let v = unsafe {
            let atom = qjs::JS_ValueToAtom(self.ctx, key.value);
            let v = qjs::JS_GetProperty(key.ctx, self.value, atom);
            qjs::JS_FreeAtom(self.ctx, atom);
            v
        };
        if unsafe { qjs::QJS_IsUndefined(self.ctx, v) != 0 } {
            None
        } else {
            Some(QJSValue::from_ffi(key.ctx, v))
        }
    }

    fn define_property(
        &self,
        key: Self,
        value: Self,
        getter: Self,
        setter: Self,
        attributes: PropertyAttributes,
    ) -> bool {
        let getter = getter.into_raw_value();
        let setter = setter.into_raw_value();
        let value = value.into_raw_value();

        let v = unsafe {
            let atom = qjs::JS_ValueToAtom(self.ctx, key.value);
            let v = qjs::JS_DefineProperty(
                self.ctx,
                self.value,
                atom,
                value,
                getter,
                setter,
                to_flags(attributes),
            );
            qjs::JS_FreeAtom(self.ctx, atom);
            v
        };
        v != 0
    }
}

fn to_flags(attr: PropertyAttributes) -> i32 {
    let mut flags = 0;

    if attr.has_value() {
        flags |= qjs::JS_PROP_HAS_VALUE;
    }

    if attr.has_get() {
        flags |= qjs::JS_PROP_HAS_GET;
    }
    if attr.has_set() {
        flags |= qjs::JS_PROP_HAS_SET;
    }

    if attr.is_writable() {
        flags |= qjs::JS_PROP_HAS_WRITABLE;
    }

    if attr.is_enumerable() {
        flags |= qjs::JS_PROP_ENUMERABLE;
    }

    if attr.is_configurable() {
        flags |= qjs::JS_PROP_CONFIGURABLE;
    }
    flags as _
}
