use crate::{qjs, QJSContext};
use rusty_js_core::{impl_js_converter, JSContextImpl, JSRawContext, JSValueImpl, RustyJSError};
use std::slice;

mod array;
mod array_buffer;
mod object;
mod typed_array;
mod valuetype;

pub struct QJSValue {
    value: qjs::JSValue,
    ctx: *mut qjs::JSContext,
    exception: bool,
}

impl Clone for QJSValue {
    fn clone(&self) -> Self {
        let value = unsafe { qjs::JS_DupValue(self.ctx, self.value) };
        Self {
            value,
            ctx: self.ctx,
            exception: self.exception,
        }
    }
}

impl Drop for QJSValue {
    fn drop(&mut self) {
        unsafe {
            // Finalizer callback, ctx is set to NULL
            if !self.ctx.is_null() {
                qjs::JS_FreeValue(self.ctx, self.value);
            }
        }
    }
}

impl JSRawContext for QJSValue {
    type RawContext = *mut qjs::JSContext;
}

impl QJSValue {
    fn _from_raw(ctx: *mut qjs::JSContext, value: qjs::JSValue) -> Self {
        // In callback context, generally, all JS variables are from JS engine, in order to make Rust lifetime
        // and ownship works, these variables should be increased referece count first, and then Rust side can
        // drop QJSValue safely
        let value = unsafe { qjs::JS_DupValue(ctx, value) };

        Self {
            value,
            ctx,
            exception: false,
        }
    }

    pub(crate) fn new(ctx: *mut qjs::JSContext, value: qjs::JSValue) -> Self {
        Self {
            value,
            ctx,
            exception: false,
        }
    }

    // In callback context, generally, all JS variables are from JS engine, in order to make Rust lifetime
    // and ownship works, these variables should be increased referece count first, and then Rust side can
    // drop QJSValue safely
    pub(crate) fn protect(mut self) -> Self {
        let value = unsafe { qjs::JS_DupValue(self.ctx, self.value) };
        self.value = value;
        self
    }

    pub(crate) fn with_exception(mut self) -> Self {
        self.exception = true;
        self
    }
}

impl JSValueImpl for QJSValue {
    type RawValue = qjs::JSValue;
    type Context = QJSContext;

    fn from_borrowed_raw(
        ctx: <Self::Context as JSContextImpl>::RawContext,
        value: Self::RawValue,
    ) -> Self {
        QJSValue::new(ctx, value).protect()
    }

    fn from_owned_raw(
        ctx: <Self::Context as JSContextImpl>::RawContext,
        value: Self::RawValue,
    ) -> Self {
        QJSValue::new(ctx, value)
    }

    fn into_raw_value(self) -> Self::RawValue {
        let value = self.value;
        std::mem::forget(self); // forbiden triggering drop
        value
    }

    fn as_raw_value(&self) -> &Self::RawValue {
        &self.value
    }

    fn as_raw_context(&self) -> &<Self::Context as JSContextImpl>::RawContext {
        &self.ctx
    }

    fn create_null(raw_ctx: &<Self::Context as JSContextImpl>::RawContext) -> Self {
        let ctx = *raw_ctx;
        let raw = unsafe { qjs::QJS_NewNull(ctx) };
        Self::from_owned_raw(ctx, raw)
    }

    fn create_undefined(raw_ctx: &<Self::Context as JSContextImpl>::RawContext) -> Self {
        let ctx = *raw_ctx;
        let raw = unsafe { qjs::QJS_NewUndefined(ctx) };
        Self::from_owned_raw(ctx, raw)
    }
}

impl QJSValue {
    // it's for debug only
    #![allow(unused)]
    pub fn get_ref_count(value: qjs::JSValue) -> i32 {
        unsafe { qjs::QJS_GetRefCount(value) }
    }
}

impl_js_converter!(
    QJSValue,
    bool,
    |ctx, value| qjs::QJS_NewBool(ctx, value as i32),
    |ctx, value, result: *mut bool| {
        if qjs::QJS_IsBool(ctx, value) > 0 {
            let status = qjs::JS_ToBool(ctx, value);
            *result = status != 0;
            0
        } else {
            -1
        }
    }
);

impl_js_converter!(
    QJSValue,
    i32,
    |ctx, value| { qjs::QJS_NewInt32(ctx, value) },
    |ctx, value, result| { qjs::JS_ToInt32(ctx, result, value) }
);

impl_js_converter!(
    QJSValue,
    u32,
    |ctx, value| { qjs::QJS_NewUint32(ctx, value) },
    |ctx, value, result| { qjs::QJS_ToUint32(ctx, result, value) }
);

impl_js_converter!(
    QJSValue,
    i64,
    |ctx, value| { qjs::QJS_NewInt64(ctx, value) },
    |ctx, value, result| { qjs::JS_ToInt64(ctx, result, value) }
);

impl_js_converter!(
    QJSValue,
    u64,
    |ctx, value| { qjs::JS_NewBigUint64(ctx, value) },
    |ctx, value, result| { qjs::JS_ToBigUint64(ctx, result, value) }
);

impl_js_converter!(
    QJSValue,
    f64,
    |ctx, val| qjs::QJS_NewFloat64(ctx, val),
    |ctx, val, result| { qjs::JS_ToFloat64(ctx, result, val) }
);

impl_js_converter!(
    QJSValue,
    &str,
    String,
    |ctx, value: &str| {
        let len = value.len();
        qjs::JS_NewStringLen(ctx, value.as_ptr() as _, len as _)
    },
    |ctx, value, result: *mut String| {
        if qjs::QJS_IsUndefined(ctx, value) != 0 {
            *result = String::from("UNDEFINED");
            return 0;
        }

        if qjs::QJS_IsString(ctx, value) == 0 {
            return -1;
        }

        let mut len: usize = 0;
        let ptr = qjs::JS_ToCStringLen2(ctx, &mut len as _, value, 0);
        if ptr.is_null() {
            return -1;
        }

        // Use from_raw_parts to get the complete string including null characters
        let slice = slice::from_raw_parts(ptr as *const u8, len);
        *result = String::from_utf8_lossy(slice).into_owned();

        qjs::JS_FreeCString(ctx, ptr);
        0
    }
);
