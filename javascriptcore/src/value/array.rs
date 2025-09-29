use crate::JSCValue;
use crate::jsc;
use rong_core::{JSArrayOps, JSValueImpl};

impl JSArrayOps for JSCValue {
    fn new(ctx: &Self::Context) -> Self {
        unsafe {
            let array = jsc::JSObjectMakeArray(
                ctx.to_raw(),
                0,                    // argumentCount
                std::ptr::null(),     // arguments
                std::ptr::null_mut(), // exception
            );
            JSCValue::from_owned_obj(ctx.to_raw(), array)
        }
    }

    fn get(&self, index: u32) -> Self {
        unsafe {
            let mut exception: jsc::JSValueRef = std::ptr::null_mut();
            let value =
                jsc::JSObjectGetPropertyAtIndex(self.ctx, self.as_obj(), index, &mut exception);
            if !exception.is_null() {
                JSCValue::from_owned_raw(self.ctx, exception).with_exception()
            } else {
                JSCValue::from_owned_raw(self.ctx, value)
            }
        }
    }

    fn set(&self, index: u32, value: Self) -> Self {
        unsafe {
            let mut exception: jsc::JSValueRef = std::ptr::null_mut();
            jsc::JSObjectSetPropertyAtIndex(
                self.ctx,
                self.as_obj(),
                index,
                value.as_value(),
                &mut exception,
            );
            if !exception.is_null() {
                JSCValue::from_owned_raw(self.ctx, exception).with_exception()
            } else {
                let raw = jsc::JSValueMakeUndefined(self.ctx);
                JSCValue::from_owned_raw(self.ctx, raw)
            }
        }
    }
}
