use crate::{jsc, JSCValue};
use rusty_js_core::JSTypeOf;

impl JSTypeOf for JSCValue {
    fn is_exception(&self) -> Option<Self> {
        if self.exception {
            Some(self.clone())
        } else {
            /*
            let obj = self.as_obj()
            let exception: *mut jsc::JSValueRef = std::ptr::null_mut();

            unsafe {
                let name_str = jsc::JSStringCreateWithUTF8CString(c"message".as_ptr() as *const _);
                let name_val = jsc::JSObjectGetProperty(self.ctx, obj, name_str, exception);
                jsc::JSStringRelease(name_str);

                if jsc::JSValueIsUndefined(self.ctx, name_val) || !exception.is_null() {
                    return None;
                }
            }
            Some(self.clone())
            */
            None
        }
    }

    fn is_error(&self) -> bool {
        if !self.is_object() {
            return false;
        }

        if self.exception {
            return true;
        }
        false

        /*
        unsafe {
            println!("......");
            let obj = self.as_obj()
            let mut exception: jsc::JSValueRef = std::ptr::null_mut();

            let name_str = jsc::JSStringCreateWithUTF8CString(c"name".as_ptr() as *const _);
            let name_val = jsc::JSObjectGetProperty(self.ctx, obj, name_str, &mut exception);
            jsc::JSStringRelease(name_str);

            if !exception.is_null() {
                println!("false: {}", line!());
                return false;
            }

            if jsc::JSValueIsString(self.ctx, name_val) {
                let name = jsc::JSValueToStringCopy(self.ctx, name_val, &mut exception);
                if !exception.is_null() {
                    jsc::JSStringRelease(name);
                    println!("false: {}", line!());
                    return false;
                }

                let name_chars = jsc::JSStringGetCharactersPtr(name);
                let length = jsc::JSStringGetLength(name);
                let name_str = std::slice::from_raw_parts(name_chars as *const u8, length);

                jsc::JSStringRelease(name);

                if let Ok(s) = std::str::from_utf8(name_str) {
                    s.ends_with("Error")
                } else {
                    false
                }
            } else {
                false
            }
        }
        */
    }

    fn is_array(&self) -> bool {
        if !self.is_object() {
            return false;
        }
        unsafe { jsc::JSValueIsArray(self.ctx, self.as_value()) }
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
        self.is_object || unsafe { jsc::JSValueIsObject(self.ctx, self.as_value()) }
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
}
