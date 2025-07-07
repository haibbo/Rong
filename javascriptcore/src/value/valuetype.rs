use crate::{JSCValue, jsc};
use rong_core::JSTypeOf;

impl JSTypeOf for JSCValue {
    fn is_exception(&self) -> bool {
        // Limitation: can not verify exception as JSCValue is from JS
        self._is_exception()
    }

    fn is_error(&self) -> bool {
        // Limitation: can not verify error as JSCValue is from JS
        if self._is_err() {
            return true;
        }

        unsafe {
            let error_ctor = crate::class::get_constructor(self.ctx, c"Error".as_ptr());
            jsc::JSValueIsInstanceOfConstructor(
                self.ctx,
                self.as_value(),
                error_ctor,
                std::ptr::null_mut(),
            )
        }
    }

    fn is_array(&self) -> bool {
        if !self.is_object() {
            return false;
        }
        unsafe { jsc::JSValueIsArray(self.ctx, self.as_value()) }
    }

    fn is_array_buffer(&self) -> bool {
        if !self.is_object() {
            return false;
        }
        unsafe {
            let error_ctor = crate::class::get_constructor(self.ctx, c"ArrayBuffer".as_ptr());
            jsc::JSValueIsInstanceOfConstructor(
                self.ctx,
                self.as_value(),
                error_ctor,
                std::ptr::null_mut(),
            )
        }
    }

    fn is_promise(&self) -> bool {
        if !self.is_object() {
            return false;
        }

        unsafe {
            let mut exception: jsc::JSValueRef = std::ptr::null_mut();
            let global_object = jsc::JSContextGetGlobalObject(self.ctx);

            // get constructor
            let promisename = jsc::JSStringCreateWithUTF8CString(c"Promise".as_ptr());
            let promise =
                jsc::JSObjectGetProperty(self.ctx, global_object, promisename, &mut exception);
            jsc::JSStringRelease(promisename);

            if !exception.is_null() {
                return false;
            }

            if !jsc::JSValueIsObject(self.ctx, promise) {
                return false;
            }

            let constructor = jsc::JSValueToObject(self.ctx, promise, &mut exception);
            if !exception.is_null() {
                return false;
            }

            // is instance of Promsie
            jsc::JSValueIsInstanceOfConstructor(
                self.ctx,
                self.as_value(),
                constructor,
                &mut exception,
            )
        }
    }

    fn is_undefined(&self) -> bool {
        unsafe { jsc::JSValueIsUndefined(self.ctx, self.as_value()) }
    }

    fn is_null(&self) -> bool {
        unsafe { jsc::JSValueIsNull(self.ctx, self.as_value()) }
    }

    fn is_boolean(&self) -> bool {
        unsafe { jsc::JSValueIsBoolean(self.ctx, self.as_value()) }
    }

    fn is_number(&self) -> bool {
        unsafe { jsc::JSValueIsNumber(self.ctx, self.as_value()) }
    }

    fn is_bigint(&self) -> bool {
        unsafe { jsc::JSValueIsBigInt(self.ctx, self.as_value()) }
    }

    fn is_string(&self) -> bool {
        unsafe { jsc::JSValueIsString(self.ctx, self.as_value()) }
    }

    fn is_symbol(&self) -> bool {
        unsafe { jsc::JSValueIsSymbol(self.ctx, self.as_value()) }
    }

    fn is_function(&self) -> bool {
        if !self.is_object() {
            return false;
        }

        unsafe {
            let obj = self.as_obj();
            jsc::JSObjectIsFunction(self.ctx, obj)
        }
    }

    fn is_object(&self) -> bool {
        self._is_object() || unsafe { jsc::JSValueIsObject(self.ctx, self.as_value()) }
    }

    fn is_constructor(&self) -> bool {
        if !self.is_object() {
            return false;
        }

        unsafe {
            let obj = self.as_obj();
            jsc::JSObjectIsConstructor(self.ctx, obj)
        }
    }

    fn is_date(&self) -> bool {
        unsafe { jsc::JSValueIsDate(self.ctx, self.as_value()) }
    }
}
