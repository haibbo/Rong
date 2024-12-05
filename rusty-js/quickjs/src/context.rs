use crate::{qjs, QJSRuntime, QJSValue};
use rusty_js_core::{JSCodeRunner, JSContextImpl, JSExceptionHandler, JSRuntimeImpl, JSValueImpl};
use std::ffi::CString;
use std::os::raw::c_char;

pub struct QJSContext {
    pub(crate) raw: *mut qjs::JSContext,
}

impl Drop for QJSContext {
    fn drop(&mut self) {
        // println!("free QJS Ctx");
        unsafe {
            qjs::JS_FreeContext(self.raw);
        }
    }
}

impl Clone for QJSContext {
    fn clone(&self) -> Self {
        Self {
            raw: unsafe { qjs::JS_DupContext(self.raw) },
        }
    }
}

impl JSContextImpl for QJSContext {
    type RawContext = *mut qjs::JSContext;
    type Runtime = QJSRuntime;

    fn new(runtime: &Self::Runtime) -> Self {
        unsafe {
            Self {
                raw: qjs::JS_NewContext(*runtime.as_raw()),
            }
        }
    }
    fn as_raw(&self) -> &Self::RawContext {
        &self.raw
    }
}

// eval option assiciated with JS_EVAL_*
#[derive(Clone, Copy)]
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
    fn to_flags(self) -> i32 {
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

impl QJSContext {
    fn eval_raw(
        &self,
        source: impl AsRef<str>,
        file_name: impl AsRef<str>,
        flags: i32,
    ) -> QJSValue {
        let source = source.as_ref();
        let c_source = CString::new(source).unwrap();
        let file_name = file_name.as_ref();
        let c_file_name = CString::new(file_name).unwrap();

        let qjs_value = unsafe {
            qjs::JS_Eval(
                self.raw,
                c_source.as_ptr(),
                source.len(),
                c_file_name.as_ptr(),
                flags,
            )
        };

        QJSValue::from_ffi(self.raw, qjs_value)
    }

    fn throw_error_internal<F>(&self, message: &str, throw_fn: F) -> QJSValue
    where
        F: FnOnce(*mut qjs::JSContext, *const c_char, *const c_char) -> qjs::JSValue,
    {
        let c_message = CString::new(message).unwrap();
        let raw = { throw_fn(self.raw, c"%s".as_ptr(), c_message.as_ptr()) };
        QJSValue::from_ffi(self.raw, raw)
    }
}

impl JSCodeRunner for QJSContext {
    type Value = QJSValue;

    fn eval(&self, source: impl AsRef<str>) -> Self::Value {
        let file_name = "eval";
        self.eval_raw(source, file_name, EvalOptions::default().to_flags())
    }

    fn global_object(&self) -> Self::Value {
        let raw = unsafe { qjs::JS_GetGlobalObject(self.raw) };
        QJSValue::from_ffi(self.raw, raw)
    }
}

impl JSExceptionHandler for QJSContext {
    type Value = QJSValue;

    fn throw_syntax_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_error_internal(message.as_ref(), |ctx, fmt, msg| unsafe {
            qjs::JS_ThrowSyntaxError(ctx, fmt, msg)
        })
    }

    fn throw_type_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_error_internal(message.as_ref(), |ctx, fmt, msg| unsafe {
            qjs::JS_ThrowTypeError(ctx, fmt, msg)
        })
    }

    fn throw_reference_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_error_internal(message.as_ref(), |ctx, fmt, msg| unsafe {
            qjs::JS_ThrowReferenceError(ctx, fmt, msg)
        })
    }

    fn throw_range_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_error_internal(message.as_ref(), |ctx, fmt, msg| unsafe {
            qjs::JS_ThrowRangeError(ctx, fmt, msg)
        })
    }

    fn throw_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_error_internal(message.as_ref(), |ctx, fmt, msg| unsafe {
            qjs::JS_ThrowPlainError(ctx, fmt, msg)
        })
    }
}
