use crate::ArkJSValue;
use crate::arkjs;
use rong_core::{JSArrayOps, JSValueImpl};

impl JSArrayOps for ArkJSValue {
    fn new_array(ctx: &Self::Context) -> Self {
        unsafe {
            let mut array: arkjs::JSVM_Value = std::ptr::null_mut();
            let status = arkjs::OH_JSVM_CreateArray(ctx.to_raw(), &mut array);
            if status == arkjs::JSVM_Status_JSVM_OK {
                ArkJSValue::from_owned_raw(ctx.to_raw(), array).with_object()
            } else {
                Self::create_undefined(ctx)
            }
        }
    }

    fn get_index(&self, index: u32) -> Self {
        unsafe {
            let mut result: arkjs::JSVM_Value = std::ptr::null_mut();
            let status = arkjs::OH_JSVM_GetElement(self.env, self.value, index, &mut result);
            if status == arkjs::JSVM_Status_JSVM_OK {
                ArkJSValue::from_owned_raw(self.env, result)
            } else {
                // Return exception
                let mut exception: arkjs::JSVM_Value = std::ptr::null_mut();
                arkjs::OH_JSVM_GetAndClearLastException(self.env, &mut exception);
                ArkJSValue::from_owned_raw(self.env, exception).with_exception()
            }
        }
    }

    fn set_index(&self, index: u32, value: Self) -> Self {
        unsafe {
            let status =
                arkjs::OH_JSVM_SetElement(self.env, self.value, index, *value.as_raw_value());

            if status == arkjs::JSVM_Status_JSVM_OK {
                let mut undefined: arkjs::JSVM_Value = std::ptr::null_mut();
                arkjs::OH_JSVM_GetUndefined(self.env, &mut undefined);
                ArkJSValue::from_owned_raw(self.env, undefined)
            } else {
                // Return exception
                let mut exception: arkjs::JSVM_Value = std::ptr::null_mut();
                arkjs::OH_JSVM_GetAndClearLastException(self.env, &mut exception);
                ArkJSValue::from_owned_raw(self.env, exception).with_exception()
            }
        }
    }
}
