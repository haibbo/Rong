use crate::{jsc, value::JSValueKind, JSCValue};
use rusty_js_core::JSTypeOf;

impl JSTypeOf for JSCValue {
    fn is_exception(&self) -> Option<Self> {
        if self.exception {
            Some(self.clone())
        } else {
            None
        }
    }

    fn is_error(&self) -> bool {
        if self.kind != JSValueKind::Object {
            return false;
        }
        false
    }

    fn is_array(&self) -> bool {
        unsafe { jsc::JSValueIsArray(self.ctx, self.value) }
    }

    fn is_promise(&self) -> bool {
        // In JavaScriptCore, no direct API. Let's assume no error
        false
    }

    fn is_undefined(&self) -> bool {
        unsafe { jsc::JSValueIsUndefined(self.ctx, self.value) }
    }

    fn is_null(&self) -> bool {
        unsafe { jsc::JSValueIsNull(self.ctx, self.value) }
    }

    fn is_boolean(&self) -> bool {
        unsafe { jsc::JSValueIsBoolean(self.ctx, self.value) }
    }

    fn is_number(&self) -> bool {
        unsafe { jsc::JSValueIsNumber(self.ctx, self.value) }
    }

    fn is_bigint(&self) -> bool {
        unsafe { jsc::JSValueIsBigInt(self.ctx, self.value) }
    }

    fn is_string(&self) -> bool {
        unsafe { jsc::JSValueIsString(self.ctx, self.value) }
    }

    fn is_symbol(&self) -> bool {
        unsafe { jsc::JSValueIsSymbol(self.ctx, self.value) }
    }

    fn is_function(&self) -> bool {
        unsafe {
            if !self.is_object() {
                return false;
            }
            jsc::JSObjectIsFunction(self.ctx, self.value as jsc::JSObjectRef)
        }
    }

    fn is_object(&self) -> bool {
        self.kind == JSValueKind::Object
    }

    fn is_constructor(&self) -> bool {
        unsafe {
            if !self.is_object() {
                return false;
            }
            jsc::JSObjectIsConstructor(self.ctx, self.value as jsc::JSObjectRef)
        }
    }
}
