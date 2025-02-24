use crate::qjs;
use crate::QJSValue;
use rusty_js_core::JSTypeOf;

impl JSTypeOf for QJSValue {
    fn is_boolean(&self) -> bool {
        unsafe { qjs::QJS_IsBool(self.ctx, self.value) != 0 }
    }

    fn is_exception(&self) -> bool {
        self._is_exception()
    }

    fn is_error(&self) -> bool {
        if self._is_err() {
            true
        } else {
            unsafe { qjs::JS_IsError(self.ctx, self.value) != 0 }
        }
    }

    fn is_array(&self) -> bool {
        unsafe { qjs::JS_IsArray(self.ctx, self.value) != 0 }
    }

    fn is_array_buffer(&self) -> bool {
        unsafe { qjs::JS_IsArrayBuffer(self.value) != 0 }
    }

    fn is_promise(&self) -> bool {
        unsafe { qjs::QJS_IsPromise(self.ctx, self.value) != 0 }
    }

    fn is_undefined(&self) -> bool {
        unsafe { qjs::QJS_IsUndefined(self.ctx, self.value) != 0 }
    }

    fn is_null(&self) -> bool {
        unsafe { qjs::QJS_IsNull(self.ctx, self.value) != 0 }
    }

    fn is_number(&self) -> bool {
        unsafe { qjs::QJS_IsNumber(self.ctx, self.value) != 0 }
    }

    fn is_bigint(&self) -> bool {
        unsafe { qjs::QJS_IsBigInt(self.ctx, self.value) != 0 }
    }

    fn is_string(&self) -> bool {
        unsafe { qjs::QJS_IsString(self.ctx, self.value) != 0 }
    }

    fn is_symbol(&self) -> bool {
        unsafe { qjs::QJS_IsSymbol(self.ctx, self.value) != 0 }
    }

    fn is_function(&self) -> bool {
        unsafe { qjs::JS_IsFunction(self.ctx, self.value) != 0 }
    }

    fn is_constructor(&self) -> bool {
        unsafe { qjs::JS_IsConstructor(self.ctx, self.value) != 0 }
    }

    fn is_object(&self) -> bool {
        unsafe { qjs::QJS_IsObject(self.ctx, self.value) != 0 }
    }
}
