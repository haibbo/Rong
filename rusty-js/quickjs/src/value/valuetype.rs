use crate::qjs;
use crate::QJSValue;
use rusty_js_core::JSTypeOf;

impl JSTypeOf for QJSValue {
    fn is_boolean(&self) -> bool {
        unsafe { qjs::QJS_IsBool(self.get_ctx(), self.get_raw()) != 0 }
    }

    fn is_exception(&self) -> bool {
        unsafe { qjs::QJS_IsException(self.get_ctx(), self.get_raw()) != 0 }
    }

    fn is_error(&self) -> bool {
        unsafe { qjs::JS_IsError(self.get_ctx(), self.get_raw()) != 0 }
    }

    fn is_array(&self) -> bool {
        unsafe { qjs::JS_IsArray(self.get_ctx(), self.get_raw()) != 0 }
    }

    fn is_promise(&self) -> bool {
        unsafe { qjs::QJS_IsPromise(self.get_ctx(), self.get_raw()) != 0 }
    }

    fn is_undefined(&self) -> bool {
        unsafe { qjs::QJS_IsUndefined(self.get_ctx(), self.get_raw()) != 0 }
    }

    fn is_null(&self) -> bool {
        unsafe { qjs::QJS_IsNull(self.get_ctx(), self.get_raw()) != 0 }
    }

    fn is_number(&self) -> bool {
        unsafe { qjs::QJS_IsNumber(self.get_ctx(), self.get_raw()) != 0 }
    }

    fn is_bigint(&self) -> bool {
        unsafe { qjs::QJS_IsBigInt(self.get_ctx(), self.get_raw()) != 0 }
    }

    fn is_string(&self) -> bool {
        unsafe { qjs::QJS_IsString(self.get_ctx(), self.get_raw()) != 0 }
    }

    fn is_symbol(&self) -> bool {
        unsafe { qjs::QJS_IsSymbol(self.get_ctx(), self.get_raw()) != 0 }
    }

    fn is_function(&self) -> bool {
        unsafe { qjs::JS_IsFunction(self.get_ctx(), self.get_raw()) != 0 }
    }

    fn is_object(&self) -> bool {
        unsafe { qjs::QJS_IsObject(self.get_ctx(), self.get_raw()) != 0 }
    }
}
