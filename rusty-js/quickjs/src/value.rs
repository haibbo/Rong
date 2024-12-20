use crate::{qjs, QJSContext};
use rusty_js_core::{impl_js_converter, JSContextImpl, JSFfiContext, JSValueImpl};
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
            // println!("Free Value");
            qjs::JS_FreeValue(self.ctx, self.value);
        }
    }
}

impl JSFfiContext for QJSValue {
    type FfiContext = *mut qjs::JSContext;
}

impl JSValueImpl for QJSValue {
    type FfiValue = qjs::JSValue;
    type Context = QJSContext;

    fn from_ffi(ctx: <Self::Context as JSContextImpl>::FfiContext, value: Self::FfiValue) -> Self {
        Self { value, ctx }
    }

    fn into_ffi_value(self) -> Self::FfiValue {
        let value = self.value;
        std::mem::forget(self); // forbiden triggering drop
        value
    }

    fn as_ffi_value(&self) -> &Self::FfiValue {
        &self.value
    }

    fn as_ffi_context(&self) -> &<Self::Context as JSContextImpl>::FfiContext {
        &self.ctx
    }
}

impl<T> From<(&T, ())> for QJSValue
where
    T: JSContextImpl<FfiContext = <QJSValue as JSFfiContext>::FfiContext>,
    QJSValue: JSValueImpl<Context = T>,
{
    fn from(t: (&T, ())) -> Self {
        let ctx = *t.0.as_ffi();
        let raw = unsafe { qjs::QJS_NewUndefined(ctx) };
        Self::from_ffi(ctx, raw)
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
