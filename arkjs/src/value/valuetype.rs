use crate::{ArkJSValue, arkjs};
use rong_core::JSTypeOf;

impl JSTypeOf for ArkJSValue {
    fn is_exception(&self) -> bool {
        // Limitation: can not verify exception as ArkJSValue is from JS
        self._is_exception()
    }

    fn is_error(&self) -> bool {
        // Limitation: can not verify error as ArkJSValue is from JS
        if self._is_err() {
            return true;
        }

        unsafe {
            let mut result = false;
            let status = arkjs::OH_JSVM_IsError(self.env, self.value, &mut result);
            status == arkjs::JSVM_Status_JSVM_OK && result
        }
    }

    fn is_array(&self) -> bool {
        if !self.is_object() {
            return false;
        }
        unsafe {
            let mut result = false;
            let status = arkjs::OH_JSVM_IsArray(self.env, self.value, &mut result);
            status == arkjs::JSVM_Status_JSVM_OK && result
        }
    }

    fn is_array_buffer(&self) -> bool {
        if !self.is_object() {
            return false;
        }
        unsafe {
            let mut result = false;
            let status = arkjs::OH_JSVM_IsArraybuffer(self.env, self.value, &mut result);
            status == arkjs::JSVM_Status_JSVM_OK && result
        }
    }

    fn is_promise(&self) -> bool {
        if !self.is_object() {
            return false;
        }

        unsafe {
            let mut result = false;
            let status = arkjs::OH_JSVM_IsPromise(self.env, self.value, &mut result);
            status == arkjs::JSVM_Status_JSVM_OK && result
        }
    }

    fn is_date(&self) -> bool {
        if !self.is_object() {
            return false;
        }

        unsafe {
            let mut result = false;
            let status = arkjs::OH_JSVM_IsDate(self.env, self.value, &mut result);
            status == arkjs::JSVM_Status_JSVM_OK && result
        }
    }

    fn is_undefined(&self) -> bool {
        unsafe {
            let mut value_type: arkjs::JSVM_ValueType = arkjs::JSVM_ValueType_JSVM_UNDEFINED;
            let status = arkjs::OH_JSVM_Typeof(self.env, self.value, &mut value_type);
            status == arkjs::JSVM_Status_JSVM_OK
                && value_type == arkjs::JSVM_ValueType_JSVM_UNDEFINED
        }
    }

    fn is_null(&self) -> bool {
        unsafe {
            let mut value_type: arkjs::JSVM_ValueType = arkjs::JSVM_ValueType_JSVM_NULL;
            let status = arkjs::OH_JSVM_Typeof(self.env, self.value, &mut value_type);
            status == arkjs::JSVM_Status_JSVM_OK && value_type == arkjs::JSVM_ValueType_JSVM_NULL
        }
    }

    fn is_boolean(&self) -> bool {
        unsafe {
            let mut value_type: arkjs::JSVM_ValueType = arkjs::JSVM_ValueType_JSVM_BOOLEAN;
            let status = arkjs::OH_JSVM_Typeof(self.env, self.value, &mut value_type);
            status == arkjs::JSVM_Status_JSVM_OK && value_type == arkjs::JSVM_ValueType_JSVM_BOOLEAN
        }
    }

    fn is_number(&self) -> bool {
        unsafe {
            let mut value_type: arkjs::JSVM_ValueType = arkjs::JSVM_ValueType_JSVM_NUMBER;
            let status = arkjs::OH_JSVM_Typeof(self.env, self.value, &mut value_type);
            status == arkjs::JSVM_Status_JSVM_OK && value_type == arkjs::JSVM_ValueType_JSVM_NUMBER
        }
    }

    fn is_bigint(&self) -> bool {
        unsafe {
            let mut value_type: arkjs::JSVM_ValueType = arkjs::JSVM_ValueType_JSVM_BIGINT;
            let status = arkjs::OH_JSVM_Typeof(self.env, self.value, &mut value_type);
            status == arkjs::JSVM_Status_JSVM_OK && value_type == arkjs::JSVM_ValueType_JSVM_BIGINT
        }
    }

    fn is_string(&self) -> bool {
        unsafe {
            let mut value_type: arkjs::JSVM_ValueType = arkjs::JSVM_ValueType_JSVM_STRING;
            let status = arkjs::OH_JSVM_Typeof(self.env, self.value, &mut value_type);
            status == arkjs::JSVM_Status_JSVM_OK && value_type == arkjs::JSVM_ValueType_JSVM_STRING
        }
    }

    fn is_symbol(&self) -> bool {
        unsafe {
            let mut value_type: arkjs::JSVM_ValueType = arkjs::JSVM_ValueType_JSVM_SYMBOL;
            let status = arkjs::OH_JSVM_Typeof(self.env, self.value, &mut value_type);
            status == arkjs::JSVM_Status_JSVM_OK && value_type == arkjs::JSVM_ValueType_JSVM_SYMBOL
        }
    }

    fn is_function(&self) -> bool {
        if !self.is_object() {
            return false;
        }

        unsafe {
            let mut value_type: arkjs::JSVM_ValueType = arkjs::JSVM_ValueType_JSVM_FUNCTION;
            let status = arkjs::OH_JSVM_Typeof(self.env, self.value, &mut value_type);
            status == arkjs::JSVM_Status_JSVM_OK
                && value_type == arkjs::JSVM_ValueType_JSVM_FUNCTION
        }
    }

    fn is_object(&self) -> bool {
        self._is_object()
            || unsafe {
                let mut value_type: arkjs::JSVM_ValueType = arkjs::JSVM_ValueType_JSVM_OBJECT;
                let status = arkjs::OH_JSVM_Typeof(self.env, self.value, &mut value_type);
                status == arkjs::JSVM_Status_JSVM_OK
                    && value_type == arkjs::JSVM_ValueType_JSVM_OBJECT
            }
    }

    fn is_constructor(&self) -> bool {
        if !self.is_object() {
            return false;
        }

        unsafe {
            let mut result = false;
            let status = arkjs::OH_JSVM_IsConstructor(self.env, self.value, &mut result);
            status == arkjs::JSVM_Status_JSVM_OK && result
        }
    }
}
