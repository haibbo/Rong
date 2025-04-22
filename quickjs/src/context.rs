use crate::{QJSRuntime, QJSValue, qjs};
use rong_core::{
    JSClass, JSContextImpl, JSExceptionHandler, JSRuntimeImpl, JSTypeOf, JSValueImpl, RongJSError,
    Source,
};
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::os::raw::c_char;

pub struct QJSContext {
    pub(crate) ctx: *mut qjs::JSContext,
}

impl Drop for QJSContext {
    fn drop(&mut self) {
        unsafe {
            qjs::JS_FreeContext(self.ctx);
        }
    }
}

impl Clone for QJSContext {
    fn clone(&self) -> Self {
        Self {
            ctx: unsafe { qjs::JS_DupContext(self.ctx) },
        }
    }
}

impl JSContextImpl for QJSContext {
    type RawContext = *mut qjs::JSContext;
    type Runtime = QJSRuntime;
    type Value = QJSValue;

    fn new(runtime: &Self::Runtime) -> Self {
        let ctx = unsafe { qjs::JS_NewContext(runtime.to_raw()) };
        Self { ctx }
    }

    fn as_raw(&self) -> &Self::RawContext {
        &self.ctx
    }

    fn from_borrowed_raw(ctx: Self::RawContext) -> Self {
        Self::_from_borrowed_raw(ctx)
    }

    fn eval(&self, source: Source) -> Self::Value {
        let options = EvalOptions::default();
        self.eval_raw(&source, options.to_flags())
    }

    fn compile_to_bytecode(&self, source: Source) -> Result<Vec<u8>, RongJSError> {
        let options = EvalOptions {
            bytecode: true,
            ..EvalOptions::default()
        };
        let obj = self.eval_raw(&source, options.to_flags());
        if obj.is_exception() {
            return Err(RongJSError::CompileToByteErr);
        }

        let mut out_size = 0;
        let slice = unsafe {
            let buf = qjs::JS_WriteObject(
                self.ctx,
                &mut out_size,
                *obj.as_raw_value(),
                qjs::JS_WRITE_OBJ_BYTECODE as _,
            );

            if buf.is_null() {
                return Err(RongJSError::CompileToByteErr);
            }

            std::slice::from_raw_parts(buf, out_size)
        };
        let bytecode = slice.to_vec();
        Ok(bytecode)
    }

    fn run_bytecode(&self, bytes: &[u8]) -> Self::Value {
        unsafe {
            let obj = qjs::JS_ReadObject(
                self.ctx,
                bytes.as_ptr(),
                bytes.len(),
                qjs::JS_READ_OBJ_BYTECODE as i32,
            );
            if qjs::QJS_IsException(self.ctx, obj) != 0 {
                QJSValue::from_owned_raw(self.ctx, obj).with_exception()
            } else {
                let eval_result = qjs::JS_EvalFunction(self.ctx, obj);
                if qjs::QJS_IsException(self.ctx, eval_result) != 0 {
                    QJSValue::from_owned_raw(self.ctx, eval_result).with_exception()
                } else {
                    QJSValue::from_owned_raw(self.ctx, eval_result)
                }
            }
        }
    }

    fn global(&self) -> Self::Value {
        let raw = unsafe { qjs::JS_GetGlobalObject(self.ctx) };
        QJSValue::from_owned_raw(self.ctx, raw)
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
        QJSValue::from_owned_raw(self.ctx, raw)
    }

    fn call(
        &self,
        function: &Self::Value,
        this: Self::Value,
        argv: Vec<Self::Value>,
    ) -> Self::Value {
        // Convert argv to raw JSValues
        let mut args: Vec<qjs::JSValue> = argv.iter().map(|v| *v.as_raw_value()).collect();

        let val = unsafe {
            qjs::JS_Call(
                self.ctx,
                *function.as_raw_value(),
                *this.as_raw_value(),
                args.len() as std::ffi::c_int,
                args.as_mut_ptr(),
            )
        };

        if unsafe { qjs::QJS_IsException(self.ctx, val) != 0 } {
            let exception = unsafe { qjs::JS_GetException(self.ctx) };
            QJSValue::from_owned_raw(self.ctx, exception).with_exception()
        } else {
            QJSValue::from_owned_raw(self.ctx, val)
        }
    }

    fn promise(&self) -> (Self::Value, Self::Value, Self::Value) {
        // Create uninitialized array
        let mut resolving_funcs = MaybeUninit::<[qjs::JSValue; 2]>::uninit();

        // Get raw pointer to the array
        let resolving_funcs_ptr = resolving_funcs.as_mut_ptr() as *mut qjs::JSValue;

        // Create promise
        let promise = unsafe { qjs::JS_NewPromiseCapability(self.ctx, resolving_funcs_ptr) };

        // Safety: JS_NewPromiseCapability initializes the array
        let resolving_funcs = unsafe { resolving_funcs.assume_init() };

        let resolve = QJSValue::from_owned_raw(self.ctx, resolving_funcs[0]);
        let reject = QJSValue::from_owned_raw(self.ctx, resolving_funcs[1]);

        (QJSValue::from_owned_raw(self.ctx, promise), resolve, reject)
    }

    fn context_id(ctx: &Self::RawContext) -> usize {
        *ctx as *const _ as usize
    }
}

impl QJSContext {
    fn _from_borrowed_raw(ctx: *mut qjs::JSContext) -> Self {
        let ctx = unsafe { qjs::JS_DupContext(ctx) };
        Self { ctx }
    }

    pub(crate) fn to_raw(&self) -> *mut qjs::JSContext {
        self.ctx
    }

    /// Converts a raw JSValue to QJSValue, handling exceptions gracefully.
    ///
    /// This function takes a raw JSValue from QuickJS and converts it into a QJSValue.
    /// If the input value represents an exception, it will be extracted and returned
    /// as a QJSValue with the exception flag set. Otherwise, a normal QJSValue will
    /// be returned.
    ///
    /// # Safety
    /// - The input `raw` must be a valid JSValue obtained from QuickJS
    /// - The context (`self`) must be valid and match the context where `raw` was created
    ///
    /// # Returns
    /// - QJSValue containing either the converted value or the exception
    pub(crate) fn to_owned_value(&self, raw: qjs::JSValue) -> QJSValue {
        let ctx = self.to_raw();
        if unsafe { qjs::QJS_IsException(ctx, raw) != 0 } {
            let exception = unsafe { qjs::JS_GetException(ctx) };
            QJSValue::from_owned_raw(ctx, exception).with_exception()
        } else {
            QJSValue::from_owned_raw(ctx, raw)
        }
    }
}

// eval option assiciated with JS_EVAL_*
#[derive(Clone, Copy)]
struct EvalOptions {
    global: bool,
    strict: bool,
    promise: bool,
    backtrace_barrier: bool,
    bytecode: bool,
}

impl Default for EvalOptions {
    fn default() -> Self {
        Self {
            global: true,
            strict: true,
            promise: false,
            bytecode: false,
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
        if self.bytecode {
            flags |= qjs::JS_EVAL_FLAG_COMPILE_ONLY;
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
            if qjs::QJS_IsException(self.ctx, val) != 0 {
                let exception = qjs::JS_GetException(self.ctx);
                QJSValue::from_owned_raw(self.ctx, exception).with_exception()
            } else {
                QJSValue::from_owned_raw(self.ctx, val)
            }
        }
    }

    fn throw_excep_internal<F>(&self, message: &str, throw_fn: F) -> QJSValue
    where
        F: FnOnce(*mut qjs::JSContext, *const c_char, *const c_char) -> qjs::JSValue,
    {
        let c_message = CString::new(message).unwrap();
        let raw = { throw_fn(self.ctx, c"%s".as_ptr(), c_message.as_ptr()) };
        QJSValue::from_owned_raw(self.ctx, raw).with_exception()
    }
}

impl JSExceptionHandler for QJSContext {
    fn throw_syntax_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_excep_internal(message.as_ref(), |ctx, fmt, msg| unsafe {
            qjs::JS_ThrowSyntaxError(ctx, fmt, msg);
            qjs::JS_GetException(ctx)
        })
    }

    fn throw_type_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_excep_internal(message.as_ref(), |ctx, fmt, msg| unsafe {
            qjs::JS_ThrowTypeError(ctx, fmt, msg);
            qjs::JS_GetException(ctx)
        })
    }

    fn throw_reference_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_excep_internal(message.as_ref(), |ctx, fmt, msg| unsafe {
            qjs::JS_ThrowReferenceError(ctx, fmt, msg);
            qjs::JS_GetException(ctx)
        })
    }

    fn throw_range_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_excep_internal(message.as_ref(), |ctx, fmt, msg| unsafe {
            qjs::JS_ThrowRangeError(ctx, fmt, msg);
            qjs::JS_GetException(ctx)
        })
    }

    fn throw_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_excep_internal(message.as_ref(), |ctx, fmt, msg| unsafe {
            qjs::JS_ThrowPlainError(ctx, fmt, msg);
            qjs::JS_GetException(ctx)
        })
        .with_exception()
    }

    fn new_error(&self) -> Self::Value {
        QJSValue::from_owned_raw(self.ctx, unsafe { qjs::JS_NewError(self.ctx) }).with_error()
    }

    fn throw(&self, value: Self::Value) -> Self::Value {
        QJSValue::from_owned_raw(self.ctx, value.into_raw_value()).with_exception()
    }
}
