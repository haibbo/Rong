use crate::{jsc, JSCContext};
use rong_js_core::{
    impl_js_converter, JSContextImpl, JSRawContext, JSTypeOf, JSValueImpl, RongJSError,
};
use std::ffi::CString;
use std::hash::Hash;

mod array;
mod array_buffer;
mod object;
mod typed_array;
mod valuetype;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) enum JSCValueType {
    Object,
    Error,
    Exception,
    Other,
}

pub struct JSCValue {
    value: *const jsc::OpaqueJSValue,
    ctx: *mut jsc::OpaqueJSContext,
    value_type: JSCValueType,
}

impl PartialEq for JSCValue {
    fn eq(&self, other: &Self) -> bool {
        unsafe { jsc::JSValueIsEqual(self.ctx, self.value, other.value, std::ptr::null_mut()) }
    }
}

impl Hash for JSCValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
        self.ctx.hash(state);
        self.value_type.hash(state);
    }
}

impl JSCValue {
    pub(crate) fn new(ctx: *mut jsc::OpaqueJSContext, value: *const jsc::OpaqueJSValue) -> Self {
        Self {
            ctx,
            value,
            value_type: JSCValueType::Other,
        }
    }

    pub(crate) fn as_obj(&self) -> jsc::JSObjectRef {
        if self.value_type == JSCValueType::Object {
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
        self.value_type = JSCValueType::Exception;
        self
    }

    pub(crate) fn with_object(mut self) -> Self {
        self.value_type = JSCValueType::Object;
        self
    }

    pub(crate) fn with_error(mut self) -> Self {
        self.value_type = JSCValueType::Error;
        self
    }

    pub(crate) fn _is_err(&self) -> bool {
        self.value_type == JSCValueType::Error
    }

    pub(crate) fn _is_exception(&self) -> bool {
        self.value_type == JSCValueType::Exception
    }

    pub(crate) fn _is_object(&self) -> bool {
        self.value_type == JSCValueType::Object
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

    fn from_json_str(ctx: &Self::Context, str: &str) -> Self {
        let ctx = ctx.to_raw();
        let c_str = CString::new(str).unwrap();
        let raw = unsafe {
            let js_string = jsc::JSStringCreateWithUTF8CString(c_str.as_ptr());
            let raw = jsc::JSValueMakeFromJSONString(ctx, js_string);
            jsc::JSStringRelease(js_string);
            raw
        };
        Self::from_owned_raw(ctx, raw)
    }

    fn create_symbol(ctx: &Self::Context, description: &str) -> Self {
        let ctx = ctx.to_raw();
        let c_str = CString::new(description).unwrap();
        let raw = unsafe {
            let js_string = jsc::JSStringCreateWithUTF8CString(c_str.as_ptr());
            let symbol = jsc::JSValueMakeSymbol(ctx, js_string);
            jsc::JSStringRelease(js_string);
            symbol
        };
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
        if !jsc::JSValueIsNumber(ctx, value) {
            return -1;
        }

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
        if !jsc::JSValueIsNumber(ctx, value) {
            return -1;
        }

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
        // This intentionally skips JSValueIsString check to allow any value that
        // the JS engine can convert to a string, providing more flexible type coercion
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
        if !jsc::JSValueIsNumber(ctx, value) {
            return -1;
        }

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
        if !jsc::JSValueIsNumber(ctx, value) && !jsc::JSValueIsBigInt(ctx, value) {
            return -1;
        }

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
        if !jsc::JSValueIsNumber(ctx, value) && !jsc::JSValueIsBigInt(ctx, value) {
            return -1;
        }

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
        cloned.value_type = self.value_type;
        cloned.protect()
    }
}
