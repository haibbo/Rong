use crate::{jsc, JSCContext};
use rusty_js_core::{impl_js_converter, JSContextImpl, JSRawContext, JSValueImpl, RustyJSError};
use std::ffi::CString;

mod array;
mod array_buffer;
mod object;
mod typed_array;
mod valuetype;

pub struct JSCValue {
    value: *const jsc::OpaqueJSValue,
    ctx: *mut jsc::OpaqueJSContext,
    pub(crate) exception: bool,
    is_object: bool,
}

impl JSCValue {
    pub(crate) fn new(ctx: *mut jsc::OpaqueJSContext, value: *const jsc::OpaqueJSValue) -> Self {
        Self {
            ctx,
            value,
            exception: false,
            is_object: false,
        }
    }

    pub(crate) fn as_obj(&self) -> jsc::JSObjectRef {
        if self.is_object {
            self.value as jsc::JSObjectRef
        } else {
            let mut exception: jsc::JSValueRef = std::ptr::null_mut();
            unsafe { jsc::JSValueToObject(self.ctx, self.value, &mut exception) }
        }
    }

    pub(crate) fn as_value(&self) -> jsc::JSValueRef {
        self.value
    }

    /// Protects the current value from garbage collection.
    pub(crate) fn protect(self) -> Self {
        unsafe {
            jsc::JSValueProtect(self.ctx, self.value);
        }
        self
    }

    pub(crate) fn with_exception(mut self) -> Self {
        self.exception = true;
        self
    }

    pub(crate) fn with_object(mut self) -> Self {
        self.is_object = true;
        self
    }

    pub(crate) fn from_borrowed_obj(ctx: *mut jsc::OpaqueJSContext, obj: jsc::JSObjectRef) -> Self {
        JSCValue::new(ctx, obj).with_object().protect()
    }

    pub(crate) fn from_owned_obj(ctx: *mut jsc::OpaqueJSContext, obj: jsc::JSObjectRef) -> Self {
        JSCValue::new(ctx, obj).with_object()
    }
}

impl Drop for JSCValue {
    fn drop(&mut self) {
        // Finalizer callback, ctx is set to NULL
        if !self.ctx.is_null() {
            unsafe {
                jsc::JSValueUnprotect(self.ctx, self.value);
            }
        }
    }
}

impl JSRawContext for JSCValue {
    type RawContext = *mut jsc::OpaqueJSContext;
}

impl JSValueImpl for JSCValue {
    type RawValue = *const jsc::OpaqueJSValue;
    type Context = JSCContext;

    fn from_borrowed_raw(
        ctx: <Self::Context as JSContextImpl>::RawContext,
        value: Self::RawValue,
    ) -> Self {
        JSCValue::new(ctx, value).protect()
    }

    fn from_owned_raw(
        ctx: <Self::Context as JSContextImpl>::RawContext,
        value: Self::RawValue,
    ) -> Self {
        JSCValue::new(ctx, value)
    }

    fn into_raw_value(self) -> Self::RawValue {
        let value = self.value;
        std::mem::forget(self); // // forbiden triggering drop
        value
    }

    fn as_raw_value(&self) -> &Self::RawValue {
        &self.value
    }

    fn as_raw_context(&self) -> &<Self::Context as JSContextImpl>::RawContext {
        &self.ctx
    }

    fn create_null(ctx: &Self::Context) -> Self {
        let ctx = ctx.to_raw();
        let raw = unsafe { jsc::JSValueMakeNull(ctx) };
        Self::from_owned_raw(ctx, raw)
    }

    fn create_undefined(ctx: &Self::Context) -> Self {
        let ctx = ctx.to_raw();
        let raw = unsafe { jsc::JSValueMakeUndefined(ctx) };
        Self::from_owned_raw(ctx, raw)
    }
}

impl_js_converter!(
    JSCValue,
    bool,
    |ctx, value| unsafe { jsc::JSValueMakeBoolean(ctx, value) },
    |ctx, value, result: *mut bool| unsafe {
        if jsc::JSValueIsBoolean(ctx, value) {
            *result = jsc::JSValueToBoolean(ctx, value);
            0
        } else {
            -1
        }
    }
);

// Note: In JavaScriptCore, all numbers are internally represented as f64 (double precision floating point).
// This means there may be precision loss when converting between integer types and JavaScript numbers.
impl_js_converter!(
    JSCValue,
    i32,
    |ctx, value| unsafe { jsc::JSValueMakeNumber(ctx, value as f64) },
    |ctx, value, result: &mut i32| unsafe {
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();
        *result = jsc::JSValueToInt32(ctx, value, &mut exception);
        if exception.is_null() {
            0
        } else {
            -1
        }
    }
);

impl_js_converter!(
    JSCValue,
    f64,
    |ctx, value| unsafe { jsc::JSValueMakeNumber(ctx, value) },
    |ctx, value, result: &mut f64| unsafe {
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();
        *result = jsc::JSValueToNumber(ctx, value, &mut exception);
        if exception.is_null() {
            0
        } else {
            -1
        }
    }
);

impl_js_converter!(
    JSCValue,
    &str,
    String,
    |ctx, value: &str| unsafe {
        let cstr = CString::new(value).unwrap();
        let js_str = jsc::JSStringCreateWithUTF8CString(cstr.as_ptr());
        let result = jsc::JSValueMakeString(ctx, js_str);
        jsc::JSStringRelease(js_str);
        result
    },
    |ctx, value, result: *mut String| unsafe {
        if jsc::JSValueIsUndefined(ctx, value) {
            *result = String::from("UNDEFINED");
            return 0;
        }

        if !jsc::JSValueIsString(ctx, value) {
            return -1;
        }

        let js_str = jsc::JSValueToStringCopy(ctx, value, std::ptr::null_mut());
        if js_str.is_null() {
            return -1;
        }

        // Get required buffer size (including null terminator)
        let max_size = jsc::JSStringGetMaximumUTF8CStringSize(js_str);
        let mut buffer: Vec<u8> = Vec::with_capacity(max_size);
        let actual_size = jsc::JSStringGetUTF8CString(
            js_str,
            buffer.as_mut_ptr().cast::<::std::os::raw::c_char>(),
            max_size,
        );
        buffer.set_len(actual_size - 1);
        jsc::JSStringRelease(js_str);

        match String::from_utf8(buffer) {
            Ok(s) => {
                *result = s.to_string();
                0
            }
            Err(_) => -1,
        }
    }
);

impl_js_converter!(
    JSCValue,
    u32,
    |ctx, value| unsafe { jsc::JSValueMakeNumber(ctx, value as f64) },
    |ctx, value, result: &mut u32| unsafe {
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();
        *result = jsc::JSValueToUInt32(ctx, value, &mut exception);
        if exception.is_null() {
            0
        } else {
            -1
        }
    }
);

// Warning: Numbers larger than 2^53 may lose precision when converted to JavaScript number
impl_js_converter!(
    JSCValue,
    i64,
    |ctx, value| unsafe { jsc::JSBigIntCreateWithInt64(ctx, value, std::ptr::null_mut()) },
    |ctx, value, result: &mut i64| unsafe {
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();
        *result = jsc::JSValueToInt64(ctx, value, &mut exception);
        if exception.is_null() {
            0
        } else {
            -1
        }
    }
);

// Warning: Numbers larger than 2^53 may lose precision when converted to JavaScript number
// Unlike QuickJS, JavaScriptCore doesn't have native BigInt support
impl_js_converter!(
    JSCValue,
    u64,
    |ctx, value| unsafe { jsc::JSBigIntCreateWithUInt64(ctx, value, std::ptr::null_mut()) },
    |ctx, value, result: &mut u64| unsafe {
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();
        *result = jsc::JSValueToUInt64(ctx, value, &mut exception);
        if exception.is_null() {
            0
        } else {
            -1
        }
    }
);

impl Clone for JSCValue {
    fn clone(&self) -> Self {
        let mut cloned = JSCValue::new(self.ctx, self.value);
        cloned.exception = self.exception;
        cloned.is_object = self.is_object;
        cloned.protect()
    }
}
