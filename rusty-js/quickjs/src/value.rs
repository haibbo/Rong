use crate::{qjs, QJSContext};
use rusty_js_core::{impl_js_converter, JSContextImpl, JSRawContext, JSValueImpl, RustyJSError};
use std::ffi::CStr;

mod object;
mod valuetype;

pub struct QJSValue {
    value: qjs::JSValue,
    ctx: *mut qjs::JSContext,
}

impl Clone for QJSValue {
    fn clone(&self) -> Self {
        let value = unsafe { qjs::JS_DupValue(self.ctx, self.value) };
        Self {
            value,
            ctx: self.ctx,
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

        Self { value, ctx }
    }
}

impl JSValueImpl for QJSValue {
    type RawValue = qjs::JSValue;
    type Context = QJSContext;

    fn from_borrowed_raw(
        ctx: <Self::Context as JSContextImpl>::RawContext,
        value: Self::RawValue,
    ) -> Self {
        QJSValue::_from_raw(ctx, value)
    }

    fn from_owned_raw(
        ctx: <Self::Context as JSContextImpl>::RawContext,
        value: Self::RawValue,
    ) -> Self {
        Self { value, ctx }
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
}

impl<T> From<(&T, ())> for QJSValue
where
    T: JSContextImpl<RawContext = <QJSValue as JSRawContext>::RawContext>,
{
    fn from(t: (&T, ())) -> Self {
        let ctx = *t.0.as_raw();
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
        let len = value.as_bytes().len();
        qjs::JS_NewStringLen(ctx, value.as_ptr() as _, len as _)
    },
    |ctx, value, result: *mut String| {
        if qjs::QJS_IsString(ctx, value) == 0 {
            return -1;
        }
        let ptr = qjs::JS_ToCStringLen2(ctx, std::ptr::null_mut(), value, 0);
        *result = CStr::from_ptr(ptr).to_string_lossy().into_owned();
        qjs::JS_FreeCString(ctx, ptr);
        0
    }
);
