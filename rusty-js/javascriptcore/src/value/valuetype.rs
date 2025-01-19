use crate::{jsc, JSCValue};
use rusty_js_core::{JSTypeOf, JSValueImpl};

impl JSTypeOf for JSCValue {
    fn is_exception(&self) -> Option<Self> {
        // In JavaScriptCore, exceptions are handled through context
        None
    }

    fn is_error(&self) -> bool {
        todo!()
        // unsafe { jsc::JSValueIsError(self.ctx, self.value) }
    }

    fn is_array(&self) -> bool {
        unsafe { jsc::JSValueIsArray(self.ctx, self.value) }
    }

    fn is_promise(&self) -> bool {
        todo!()
        // unsafe { jsc::JSValueIsPromise(self.ctx, self.value) }
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
        todo!()
        // unsafe { jsc::JSValueIsFunction(self.ctx, self.value) }
    }

    fn is_object(&self) -> bool {
        unsafe { jsc::JSValueIsObject(self.ctx, self.value) }
    }

    fn is_constructor(&self) -> bool {
        todo!()
        // unsafe { jsc::JSValueIsConstructor(self.ctx, self.value) }
    }
}
