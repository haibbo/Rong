use crate::{qjs, QJSContext};
use rusty_js_core::{impl_js_converter, JSContextImpl, JSRawContext, JSValueError, JSValueImpl};
use std::ffi::CStr;

mod object;
mod valuetype;

pub struct QJSValue {
    pub(crate) value: qjs::JSValue,
    pub(crate) ctx: *mut qjs::JSContext,
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

impl JSRawContext for QJSValue {
    type RawContext = *mut qjs::JSContext;
}

impl JSValueImpl for QJSValue {
    type RawValue = qjs::JSValue;
    type Context = QJSContext;

    fn from_ffi(
        ctx_raw: <Self::Context as JSContextImpl>::RawContext,
        value_raw: Self::RawValue,
    ) -> Self {
        Self {
            value: value_raw,
            ctx: ctx_raw,
        }
    }

    fn as_raw_value(&self) -> &Self::RawValue {
        &self.value
    }

    fn as_raw_context(&self) -> &<Self::Context as JSContextImpl>::RawContext {
        &self.ctx
    }
}

// use default implementation
impl JSValueError for QJSValue {}

impl TryInto<()> for QJSValue
where
    QJSValue: JSValueImpl,
{
    type Error = String;
    fn try_into(self) -> Result<(), Self::Error> {
        Ok(())
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
