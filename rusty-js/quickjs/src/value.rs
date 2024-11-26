use crate::qjs;
use crate::JSCtxInner;
use rusty_js_traits::{impl_js_value, ExtractJSError, FromHost, FromRaw, IntoHost};
use std::ffi::CStr;
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

impl<'ctx> ExtractJSError for JSValueInner<'ctx> {
    fn extract_err_msg(&self) -> String {
        let excep_val = unsafe { qjs::JS_GetException(self.ctx.as_ptr()) };
        let err_val = unsafe { qjs::JS_DupValue(self.ctx.as_ptr(), excep_val) };

        let value = JSValueInner::from_raw(self.ctx, excep_val);
        let mut err_msg: String = value.into_host().unwrap();

        unsafe {
            if qjs::JS_IsError(self.ctx.as_ptr(), err_val) > 0 {
                let cstr = CStr::from_bytes_with_nul(b"stack\0").unwrap();
                let val = qjs::JS_GetPropertyStr(self.ctx.as_ptr(), err_val, cstr.as_ptr());

                if qjs::QJS_IsUndefined(self.ctx.as_ptr(), val) == 0 {
                    let stack = JSValueInner::from_raw(self.ctx, val);
                    let stack_msg: String = stack.into_host().unwrap();

                    err_msg.push_str("\nstack:\n");
                    err_msg.push_str(&stack_msg);
                }
            }
            qjs::JS_FreeValue(self.ctx.as_ptr(), err_val);
        }
        err_msg
    }
}

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
    &str,
    |ctx, value: &str| {
        let len = value.as_bytes().len();
        qjs::JS_NewStringLen(ctx, value.as_ptr() as _, len as _)
    },
    |ctx, result: *mut String, value| {
        if qjs::QJS_IsString(ctx, value) < 0 {
            -1
        } else {
            let ptr = qjs::JS_ToCStringLen2(ctx, std::ptr::null_mut(), value, 0);
            *result = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            qjs::JS_FreeCString(ctx, ptr);
            0
        }
    },
    String
);
