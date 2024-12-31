use crate::{qjs, QJSRuntime, QJSValue};
use rusty_js_core::{
    JSClass, JSCodeRunner, JSContextImpl, JSExceptionHandler, JSRuntimeImpl, JSValueImpl, Source,
};
use std::ffi::CString;
use std::os::raw::{c_char, c_void};

pub struct QJSContext {
    pub(crate) ctx: *mut qjs::JSContext,
}

impl Drop for QJSContext {
    fn drop(&mut self) {
        // println!("free QJS Ctx");
        unsafe {
            qjs::JS_FreeContext(self.ctx);
        }
    }
}

impl Clone for QJSContext {
    fn clone(&self) -> Self {
        // println!("clone QJS Ctx");
        Self {
            ctx: unsafe { qjs::JS_DupContext(self.ctx) },
        }
    }
}

impl JSContextImpl for QJSContext {
    type FfiContext = *mut qjs::JSContext;
    type Runtime = QJSRuntime;
    type Value = QJSValue;

    fn new(runtime: &Self::Runtime) -> Self {
        unsafe {
            Self {
                ctx: qjs::JS_NewContext(runtime.to_ffi()),
            }
        }
    }
    fn to_ffi(&self) -> Self::FfiContext {
        self.ctx
    }

    fn from_ffi(ctx: Self::FfiContext) -> Self {
        Self::_from_ffi(ctx)
    }

    /// Set opaque data for the context
    fn set_opaque<T>(&self, data: *mut T) {
        unsafe { qjs::JS_SetContextOpaque(self.ctx, data as *mut c_void) };
    }

    /// Get opaque data from the context
    fn get_opaque<T>(&self) -> *mut T {
        unsafe { qjs::JS_GetContextOpaque(self.ctx) as *mut T }
    }
}

impl QJSContext {
    fn _from_ffi(ctx: *mut qjs::JSContext) -> Self {
        let ctx = unsafe { qjs::JS_DupContext(ctx) };
        Self { ctx }
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
    fn eval_raw(&self, source: &Source, flags: i32) -> QJSValue {
        let filename = source.name().unwrap_or("eval");
        let c_code = CString::new(source.code()).unwrap();
        let c_filename = CString::new(filename).unwrap();

        unsafe {
            let val = qjs::JS_Eval(
                self.ctx,
                c_code.as_ptr(),
                c_code.as_bytes().len(),
                c_filename.as_ptr(),
                flags,
            );
            QJSValue::from_ffi(self.ctx, val)
        }
    }

    fn throw_error_internal<F>(&self, message: &str, throw_fn: F) -> QJSValue
    where
        F: FnOnce(*mut qjs::JSContext, *const c_char, *const c_char) -> qjs::JSValue,
    {
        let message = message.replace("\\n", "\n");
        let c_message = CString::new(message).unwrap();
        let raw = { throw_fn(self.ctx, c"%s".as_ptr(), c_message.as_ptr()) };
        QJSValue::from_ffi(self.ctx, raw)
    }
}

impl JSCodeRunner for QJSContext {
    fn eval(&self, source: Source) -> Self::Value {
        let options = EvalOptions::default();
        self.eval_raw(&source, options.to_flags())
    }

    fn global_object(&self) -> Self::Value {
        let raw = unsafe { qjs::JS_GetGlobalObject(self.ctx) };
        QJSValue::from_ffi(self.ctx, raw)
    }

    fn register_class<JC>(&self) -> Self::Value
    where
        JC: JSClass<QJSValue>,
    {
        let name = CString::new(JC::NAME).unwrap();
        let raw = unsafe {
            qjs::QJS_CreateClass(
                self.ctx,
                name.as_ptr(),
                Some(crate::class::generic_constructor::<JC>),
                Some(crate::class::call::<JC>),
                Some(crate::class::finalizer::<JC>),
            )
        };
        QJSValue::from_ffi(self.ctx, raw)
    }
}

impl JSExceptionHandler for QJSContext {
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

    fn new_error(&self) -> Self::Value {
        unsafe { QJSValue::from_ffi(self.ctx, qjs::JS_NewError(self.ctx)) }
    }
}
