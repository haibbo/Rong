use crate::qjs;
use crate::runtime::JSRtInner;
use crate::value::JSValueInner;
use anyhow;
use rusty_js_traits::{FromRaw, IntoHost, JSCtxExt, JSError};
use std::ffi::CString;
use std::ptr::NonNull;

pub struct JSCtxInner {
    ctx: NonNull<qjs::JSContext>,
}

impl JSCtxInner {
    pub fn new(rt: &JSRtInner) -> Result<Self, String> {
        let ctx_ptr = unsafe { qjs::JS_NewContext(rt.0.as_ptr()) };
        let ctx =
            NonNull::new(ctx_ptr).ok_or_else(|| String::from("Failed to create JSContext"))?;
        Ok(Self { ctx })
    }

    pub fn as_ptr(&self) -> *mut qjs::JSContext {
        self.ctx.as_ptr()
    }

    pub fn from_ffi(ctx: *mut qjs::JSContext) -> Self {
        let ctx = unsafe {
            qjs::JS_DupContext(ctx);
            NonNull::new_unchecked(ctx)
        };
        JSCtxInner { ctx }
    }
}

impl Drop for JSCtxInner {
    fn drop(&mut self) {
        unsafe {
            qjs::JS_FreeContext(self.ctx.as_ptr());
        }
    }
}

impl Clone for JSCtxInner {
    fn clone(&self) -> Self {
        let ctx = unsafe {
            let ctx = qjs::JS_DupContext(self.ctx.as_ptr());
            NonNull::new_unchecked(ctx)
        };
        JSCtxInner { ctx }
    }
}

// eval option assiciated with JS_EVAL_*
struct EvalOptions {
    global: bool,
    strict: bool,
    promise: bool,
    backtrace_barrier: bool,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self {
            global: true,
            strict: true,
            promise: false,
            backtrace_barrier: false,
        }
    }
}

impl EvalOptions {
    fn to_flags(&self) -> i32 {
        let mut flags = if self.global {
            qjs::JS_EVAL_TYPE_GLOBAL
        } else {
            qjs::JS_EVAL_TYPE_MODULE
        };

        if self.strict {
            flags |= qjs::JS_EVAL_FLAG_STRICT;
        }

        if self.promise {
            flags |= qjs::JS_EVAL_FLAG_ASYNC;
        }
        if self.backtrace_barrier {
            flags |= qjs::JS_EVAL_FLAG_BACKTRACE_BARRIER;
        }
        flags as _
    }
}

impl<'ctx> JSCtxInner {
    fn eval_raw(
        &'ctx self,
        source: impl AsRef<str>,
        file_name: impl AsRef<str>,
        flags: i32,
    ) -> anyhow::Result<JSValueInner<'ctx>> {
        let source = source.as_ref();
        let script = CString::new(source).unwrap();
        let file_name = file_name.as_ref();
        let file_name = CString::new(file_name).unwrap();

        let js_value = unsafe {
            qjs::JS_Eval(
                self.ctx.as_ptr(),
                script.as_ptr(),
                source.len(),
                file_name.as_ptr(),
                flags,
            )
        };
        let value = JSValueInner::from_raw(self, js_value);
        if unsafe { qjs::QJS_IsException(self.ctx.as_ptr(), js_value) > 0 } {
            let err = JSError::new(&value);
            Err(anyhow::Error::new(err))
        } else {
            Ok(value)
        }
    }
}

impl<'ctx> JSCtxExt<'ctx> for JSCtxInner {
    type Value = JSValueInner<'ctx>;
    fn eval<S, T>(&'ctx self, source: S) -> anyhow::Result<T>
    where
        S: AsRef<str>,
        Self::Value: IntoHost<T>,
    {
        let file_name = "eval";
        let value = self.eval_raw(source, file_name, EvalOptions::default().to_flags())?;
        let result: Option<T> = value.into_host();
        Ok(result.unwrap())
    }
}
