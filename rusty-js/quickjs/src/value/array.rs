use crate::qjs;
use crate::QJSValue;
use rusty_js_core::{JSArrayOps, JSValueImpl};

impl JSArrayOps for QJSValue {
    fn new(ctx: &Self::Context) -> Self {
        let ctx = ctx.to_raw();
        let v = unsafe { qjs::JS_NewArray(ctx) };
        QJSValue::from_owned_raw(ctx, v)
    }

    fn get(&self, index: u32) -> Self {
        let v = unsafe { qjs::JS_GetPropertyUint32(self.ctx, self.value, index) };
        QJSValue::from_owned_raw(self.ctx, v)
    }

    fn set(&self, index: u32, value: Self) {
        unsafe {
            qjs::JS_SetPropertyUint32(self.ctx, self.value, index, value.into_raw_value());
        }
    }
}
