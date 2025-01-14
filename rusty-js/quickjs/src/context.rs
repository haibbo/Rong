use crate::{qjs, QJSRuntime, QJSValue};
use rusty_js_core::{
    JSClass, JSContextImpl, JSExceptionHandler, JSRuntimeImpl, JSValueImpl, Source,
};
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::os::raw::{c_char, c_void};
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "ref_count_tracking")]
macro_rules! ref_count_println {
    ($($arg:tt)*) => (println!($($arg)*));
}

#[cfg(not(feature = "ref_count_tracking"))]
macro_rules! ref_count_println {
    ($($arg:tt)*) => {};
}

/// Container to hold the context-specific data for a QJSContext.
///
/// # Fields
/// - `registry`: A pointer to a RefCell containing a HashMap that maps TypeId to QJSValue
/// - `ref_count`: An AtomicUsize to track the reference count of the context
struct ContextData {
    registry: *mut RefCell<HashMap<TypeId, QJSValue>>,
    ref_count: AtomicUsize,
}

impl ContextData {
    fn new(registry: *mut RefCell<HashMap<TypeId, QJSValue>>) -> Box<Self> {
        Box::new(Self {
            registry,
            ref_count: AtomicUsize::new(1),
        })
    }

    fn increment_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
    }

    fn decrement_ref(&self) -> bool {
        self.ref_count.fetch_sub(1, Ordering::SeqCst) == 1
    }
}

pub struct QJSContext {
    pub(crate) ctx: *mut qjs::JSContext,
}

impl Drop for QJSContext {
    fn drop(&mut self) {
        let data = unsafe { qjs::JS_GetContextOpaque(self.ctx) as *mut ContextData };

        if !data.is_null() {
            unsafe {
                // If it's the last reference, clean up registry and ContextData
                if (*data).decrement_ref() {
                    ref_count_println!("free registry on last drop (ref_count: 0)");

                    Self::free_class_registry((*data).registry);
                    let _ = Box::from_raw(data);
                } else {
                    ref_count_println!(
                        "skip free registry on drop (ref_count: {})",
                        (*data).ref_count.load(Ordering::SeqCst)
                    );
                }
            }
        }

        unsafe {
            qjs::JS_FreeContext(self.ctx);
        }
    }
}

impl Clone for QJSContext {
    fn clone(&self) -> Self {
        let data = unsafe { qjs::JS_GetContextOpaque(self.ctx) as *mut ContextData };

        if !data.is_null() {
            unsafe {
                (*data).increment_ref();
                ref_count_println!(
                    "increment ref on clone (ref_count: {})",
                    (*data).ref_count.load(Ordering::SeqCst)
                );
            }
        }

        Self {
            ctx: unsafe { qjs::JS_DupContext(self.ctx) },
        }
    }
}

impl JSContextImpl for QJSContext {
    type FfiContext = *mut qjs::JSContext;
    type Runtime = QJSRuntime;
    type Value = QJSValue;

    fn new(runtime: &Self::Runtime, registry: *mut RefCell<HashMap<TypeId, Self::Value>>) -> Self {
        let ctx = unsafe { qjs::JS_NewContext(runtime.to_ffi()) };

        let data = ContextData::new(registry);
        unsafe {
            qjs::JS_SetContextOpaque(ctx, Box::into_raw(data) as *mut c_void);
        }

        Self { ctx }
    }

    fn to_ffi(&self) -> Self::FfiContext {
        self.ctx
    }

    fn from_ffi(ctx: Self::FfiContext) -> Self {
        Self::_from_ffi(ctx)
    }

    fn get_class_registry(&self) -> Option<&RefCell<HashMap<TypeId, Self::Value>>> {
        let data = unsafe { qjs::JS_GetContextOpaque(self.ctx) as *mut ContextData };
        if data.is_null() {
            None
        } else {
            unsafe {
                let registry = (*data).registry;
                if registry.is_null() {
                    None
                } else {
                    Some(&*registry)
                }
            }
        }
    }

    fn eval(&self, source: Source) -> Self::Value {
        let options = EvalOptions::default();
        self.eval_raw(&source, options.to_flags())
    }

    fn global(&self) -> Self::Value {
        let raw = unsafe { qjs::JS_GetGlobalObject(self.ctx) };
        QJSValue::from_parts(self.ctx, raw)
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
        QJSValue::from_parts(self.ctx, raw)
    }

    fn call(
        &self,
        function: &Self::Value,
        this: Option<Self::Value>,
        argv: Vec<Self::Value>,
    ) -> Self::Value {
        // Convert this to JSValue or undefined
        let this_val = this.map_or_else(
            || unsafe { qjs::QJS_NewUndefined(self.ctx) },
            |v| *v.as_ffi_value(),
        );

        // Convert argv to raw JSValues
        let mut args: Vec<qjs::JSValue> = argv.iter().map(|v| *v.as_ffi_value()).collect();

        let v = unsafe {
            qjs::JS_Call(
                self.ctx,
                *function.as_ffi_value(),
                this_val,
                args.len() as std::ffi::c_int,
                args.as_mut_ptr(),
            )
        };
        QJSValue::from_parts(self.ctx, v)
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

        let resolve = QJSValue::from_parts(self.ctx, resolving_funcs[0]);
        let reject = QJSValue::from_parts(self.ctx, resolving_funcs[1]);

        (QJSValue::from_parts(self.ctx, promise), resolve, reject)
    }
}

impl QJSContext {
    fn _from_ffi(ctx: *mut qjs::JSContext) -> Self {
        let data = unsafe { qjs::JS_GetContextOpaque(ctx) as *mut ContextData };

        if !data.is_null() {
            unsafe {
                (*data).increment_ref();
                ref_count_println!(
                    "increment ref on from_ffi (ref_count: {})",
                    (*data).ref_count.load(Ordering::SeqCst)
                );
            }
        }

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
            QJSValue::from_parts(self.ctx, val)
        }
    }

    fn throw_error_internal<F>(&self, message: &str, throw_fn: F) -> QJSValue
    where
        F: FnOnce(*mut qjs::JSContext, *const c_char, *const c_char) -> qjs::JSValue,
    {
        let c_message = CString::new(message).unwrap();
        let raw = { throw_fn(self.ctx, c"%s".as_ptr(), c_message.as_ptr()) };
        QJSValue::from_parts(self.ctx, raw)
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
        unsafe { QJSValue::from_parts(self.ctx, qjs::JS_NewError(self.ctx)) }
    }
}
