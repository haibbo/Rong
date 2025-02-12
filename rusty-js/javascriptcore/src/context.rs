use crate::{jsc, JSCRuntime, JSCValue};
use rusty_js_core::{
    JSClass, JSContextImpl, JSExceptionHandler, JSRuntimeImpl, JSValueImpl, RustyJSError,
};
use std::ffi::CString;
use std::ptr;

pub struct JSCContext {
    raw: *mut jsc::OpaqueJSContext,
}

impl JSContextImpl for JSCContext {
    type RawContext = *mut jsc::OpaqueJSContext;
    type Runtime = JSCRuntime;
    type Value = JSCValue;

    fn new(runtime: &Self::Runtime) -> Self {
        Self {
            raw: unsafe { jsc::JSGlobalContextCreateInGroup(runtime.to_raw(), ptr::null_mut()) },
        }
    }

    fn as_raw(&self) -> &Self::RawContext {
        &self.raw
    }

    fn context_id(ctx: &Self::RawContext) -> usize {
        *ctx as *const _ as usize
    }

    fn from_borrowed_raw(ctx: Self::RawContext) -> Self {
        Self::_from_borrowed_raw(ctx)
    }

    fn eval(&self, source: rusty_js_core::Source) -> Self::Value {
        let filename = source.name().unwrap_or("eval");
        let code = CString::new(source.code()).unwrap();
        let c_filename = CString::new(filename).unwrap();

        unsafe {
            let js_str = jsc::JSStringCreateWithUTF8CString(code.as_ptr());
            let js_filename = jsc::JSStringCreateWithUTF8CString(c_filename.as_ptr());

            let mut exception: jsc::JSValueRef = ptr::null_mut();
            let result = jsc::JSEvaluateScript(
                self.raw,
                js_str,
                std::ptr::null_mut(), // thisObject (null means use global object)
                js_filename,
                1,
                &mut exception,
            );

            jsc::JSStringRelease(js_str);
            jsc::JSStringRelease(js_filename);

            // Check if an exception occurred
            if !exception.is_null() {
                // println!("got exception");
                return JSCValue::from_owned_raw(self.raw, exception).with_exception();
            }
            // println!(
            //     "Result isObject: {}",
            //     jsc::JSValueIsObject(self.raw, result)
            // );
            JSCValue::from_owned_raw(self.raw, result)
        }
    }

    fn global(&self) -> Self::Value {
        unsafe {
            let global_obj = jsc::JSContextGetGlobalObject(self.raw);
            JSCValue::from_owned_obj(self.raw, global_obj)
        }
    }

    /// Bug:
    ///
    /// It is not possible to use JS subclassing with objects created from a class
    /// definition that sets callAsConstructor by default. The callAsConstructor's
    /// constructor is not changed to extended class constructor.
    ///
    /// Subclassing is supported via the JSObjectMakeConstructor function, but it has
    /// disadvantages:
    /// - can not set private data to constructor object. WeakMap is alternative solution.
    /// - typeof constructor is 'object' not 'function'(nodejs, bun, quickjs etc)
    ///
    /// Because the crate is specially desigend for mini-program, we shoud not avoid
    /// to use subclass/extend-class, in other word, directly extend feature at native
    /// side.
    fn register_class<JC>(&self) -> Self::Value
    where
        JC: JSClass<Self::Value>,
    {
        crate::class::register_class_internal::<JC>(self, JC::NAME)
    }

    fn call(
        &self,
        function: &Self::Value,
        this: Option<Self::Value>,
        argv: Vec<Self::Value>,
    ) -> Self::Value {
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();

        let this_obj = this.map_or_else(
            || unsafe { jsc::JSValueMakeUndefined(self.raw) },
            |v| {
                let raw = *v.as_raw_value();
                raw.cast()
            },
        );

        // Convert argv to raw JSValues
        let args: Vec<jsc::JSValueRef> = argv
            .iter()
            .map(|v| {
                let raw = *v.as_raw_value();
                raw.cast()
            })
            .collect();

        let result = unsafe {
            jsc::JSObjectCallAsFunction(
                self.raw,
                function.as_obj(),
                this_obj as jsc::JSObjectRef,
                args.len(),
                args.as_ptr(),
                &mut exception,
            )
        };

        if !exception.is_null() {
            return JSCValue::from_owned_raw(self.raw, exception).with_exception();
        }

        JSCValue::from_owned_raw(self.raw, result)
    }

    fn promise(&self) -> (Self::Value, Self::Value, Self::Value) {
        unsafe {
            let mut resolve_fn: jsc::JSObjectRef = std::ptr::null_mut();
            let mut reject_fn: jsc::JSObjectRef = std::ptr::null_mut();
            let mut exception: jsc::JSValueRef = std::ptr::null_mut();

            let promise = jsc::JSObjectMakeDeferredPromise(
                self.raw,
                &mut resolve_fn,
                &mut reject_fn,
                &mut exception,
            );

            if !exception.is_null() {
                let undefined = jsc::JSValueMakeUndefined(self.raw);
                return (
                    JSCValue::from_owned_raw(self.raw, undefined),
                    JSCValue::from_owned_raw(self.raw, undefined),
                    JSCValue::from_owned_raw(self.raw, undefined),
                );
            }

            (
                JSCValue::from_owned_obj(self.raw, promise),
                JSCValue::from_owned_obj(self.raw, resolve_fn),
                JSCValue::from_owned_obj(self.raw, reject_fn),
            )
        }
    }

    fn compile_to_bytecode(&self, _source: rusty_js_core::Source) -> Result<Vec<u8>, RustyJSError> {
        Err(RustyJSError::NotSupportByteCode)
    }

    fn run_bytecode(&self, _bytes: &[u8]) -> Self::Value {
        todo!()
    }
}

impl JSCContext {
    fn _from_borrowed_raw(ctx: *mut jsc::OpaqueJSContext) -> Self {
        let raw = unsafe { jsc::JSGlobalContextRetain(ctx) };
        Self { raw }
    }

    pub(crate) fn to_raw(&self) -> *mut jsc::OpaqueJSContext {
        self.raw
    }
}

impl Drop for JSCContext {
    fn drop(&mut self) {
        unsafe {
            jsc::JSGlobalContextRelease(self.raw);
        }
    }
}

impl Clone for JSCContext {
    fn clone(&self) -> Self {
        unsafe {
            // Retains a global JavaScript execution context.
            jsc::JSGlobalContextRetain(self.raw);
            Self { raw: self.raw }
        }
    }
}

impl JSExceptionHandler for JSCContext {
    fn throw_syntax_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_error_with_name("SyntaxError", message)
    }

    fn throw_type_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_error_with_name("TypeError", message)
    }

    fn throw_reference_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_error_with_name("ReferenceError", message)
    }

    fn throw_range_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_error_with_name("RangeError", message)
    }

    fn throw_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_error_with_name("Error", message)
    }

    fn new_error(&self) -> Self::Value {
        unsafe {
            let args = [];
            let error = jsc::JSObjectMakeError(self.raw, 0, args.as_ptr(), ptr::null_mut());
            JSCValue::from_owned_obj(self.raw, error)
        }
    }
}

impl JSCContext {
    /// Helper function to throw an error with a specific name
    pub(crate) fn throw_error_with_name(
        &self,
        error_name: &str,
        message: impl AsRef<str>,
    ) -> JSCValue {
        unsafe {
            // Escape single quotes in message
            let message = message.as_ref().replace('\'', "\\'");

            // Create simple eval string
            let eval_str = format!(
                "new {error_name}('{message}')",
                error_name = error_name,
                message = message
            );
            println!("xx: {}", eval_str);
            let c_eval = CString::new(eval_str).unwrap();
            let js_str = jsc::JSStringCreateWithUTF8CString(c_eval.as_ptr());

            let mut exception: jsc::JSValueRef = ptr::null_mut();
            let error = jsc::JSEvaluateScript(
                self.raw,
                js_str,
                ptr::null_mut(),
                ptr::null_mut(),
                1,
                &mut exception,
            );

            jsc::JSStringRelease(js_str);

            let error = JSCValue::from_owned_raw(self.raw, error);
            error.with_exception()
        }
    }
}
