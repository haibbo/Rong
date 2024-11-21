use crate::qjs;
use crate::JSCtxInner;
use rusty_js_traits::{impl_js_value, FromRaw, FromWithCtx};
use std::ffi::{CStr, CString};
use std::string::String;

// JSValue's lifetime depends on JSCtxInner
pub struct JSValueInner<'ctx> {
    value: qjs::JSValue,
    ctx: &'ctx JSCtxInner,
}

impl<'ctx> FromRaw<'ctx, qjs::JSValue> for JSValueInner<'ctx> {
    type Context = JSCtxInner;
    fn from_raw(ctx: &'ctx Self::Context, value: qjs::JSValue) -> Self {
        Self { ctx, value }
    }
}

impl<'ctx> Drop for JSValueInner<'ctx> {
    fn drop(&mut self) {
        unsafe {
            qjs::JS_FreeValue(self.ctx.as_ptr(), self.value);
        }
    }
}

impl<'ctx> Clone for JSValueInner<'ctx> {
    fn clone(&self) -> Self {
        Self {
            value: unsafe { qjs::JS_DupValue(self.ctx.as_ptr(), self.value) },
            ctx: self.ctx,
        }
    }
}

impl<'ctx> PartialEq for JSValueInner<'ctx> {
    fn eq(&self, other: &Self) -> bool {
        let value = unsafe { qjs::JS_IsSameValueZero(self.ctx.as_ptr(), self.value, other.value) };
        value != 0
    }
}

impl<'ctx> Eq for JSValueInner<'ctx> {}

impl_js_value!(
    bool,
    |ctx, value| qjs::QJS_NewBool(ctx, value as i32),
    |ctx, result: *mut bool, value| {
        if qjs::QJS_IsBool(ctx, value) > 0 {
            let status = qjs::JS_ToBool(ctx, value);
            *result = status != 0;
            0
        } else {
            -1
        }
    }
);

impl_js_value!(
    i32,
    |ctx, value| { qjs::QJS_NewInt32(ctx, value) },
    |ctx, result, value| { qjs::JS_ToInt32(ctx, result, value) }
);

impl_js_value!(
    u32,
    |ctx, value| { qjs::QJS_NewUint32(ctx, value) },
    |ctx, result, value| { qjs::QJS_ToUint32(ctx, result, value) }
);

impl_js_value!(
    i64,
    |ctx, value| { qjs::QJS_NewInt64(ctx, value) },
    |ctx, result, value| { qjs::JS_ToInt64(ctx, result, value) }
);

impl_js_value!(
    u64,
    |ctx, value| { qjs::JS_NewBigUint64(ctx, value) },
    |ctx, result, value| { qjs::JS_ToBigUint64(ctx, result, value) }
);

impl_js_value!(
    f64,
    |ctx, value| { qjs::QJS_NewFloat64(ctx, value) },
    |ctx, result, value| { qjs::JS_ToFloat64(ctx, result, value) }
);

impl_js_value!(
    String,
    |ctx, value| {
        let c_str = CString::new(value).unwrap();
        qjs::JS_NewStringLen(ctx, c_str.as_ptr(), c_str.count_bytes())
    },
    |ctx, result: *mut String, value| {
        if qjs::QJS_IsString(ctx, value) < 0 {
            -1
        } else {
            let ptr = qjs::JS_ToCStringLen2(ctx, std::ptr::null_mut(), value, 0);
            *result = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            0
        }
    }
);
