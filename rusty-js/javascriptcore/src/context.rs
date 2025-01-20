use crate::{jsc, JSCRuntime, JSCValue};
use rusty_js_core::{JSContextImpl, JSRuntimeImpl, JSValueImpl};
use std::ffi::CString;

pub struct JSCContext {
    raw: *mut jsc::OpaqueJSContext,
}

impl JSContextImpl for JSCContext {
    type RawContext = *mut jsc::OpaqueJSContext;
    type Runtime = JSCRuntime;
    type Value = JSCValue;

    fn new(runtime: &Self::Runtime) -> Self {
        Self {
            raw: unsafe {
                jsc::JSGlobalContextCreateInGroup(runtime.to_raw(), std::ptr::null_mut())
            },
        }
    }

    fn as_raw(&self) -> &Self::RawContext {
        &self.raw
    }

    fn context_id(ctx: &Self::RawContext) -> usize {
        *ctx as *const _ as usize
    }

    fn from_borrowed_raw(ctx: Self::RawContext) -> Self {
        todo!()
    }

    fn eval(&self, source: rusty_js_core::Source) -> Self::Value {
        let filename = source.name().unwrap_or("eval");
        let code = CString::new(source.code()).unwrap();

        unsafe {
            let js_str = jsc::JSStringCreateWithUTF8CString(code.as_ptr() as *const _);
            let js_filename = jsc::JSStringCreateWithUTF8CString(filename.as_ptr() as *const _);

            let exception: *mut jsc::JSValueRef = std::ptr::null_mut();
            let result = jsc::JSEvaluateScript(
                self.raw,
                js_str,
                std::ptr::null_mut(), // thisObject (null means use global object)
                js_filename,
                1,
                exception,
            );

            jsc::JSStringRelease(js_str);
            jsc::JSStringRelease(js_filename);

            // Check if an exception occurred
            if !exception.is_null() {
                // Convert the exception to a JSValue
                return JSCValue::from_owned_raw(self.raw, *exception);
            }

            JSCValue::from_owned_raw(self.raw, result)
        }
    }

    fn global(&self) -> Self::Value {
        unsafe {
            let global_obj = jsc::JSContextGetGlobalObject(self.raw);
            JSCValue::from_owned_raw(self.raw, global_obj)
        }
    }

    fn register_class<JC>(&self) -> Self::Value
    where
        JC: rusty_js_core::JSClass<Self::Value>,
    {
        todo!()
    }

    fn call(
        &self,
        function: &Self::Value,
        this: Option<Self::Value>,
        argv: Vec<Self::Value>,
    ) -> Self::Value {
        todo!()
    }

    fn promise(&self) -> (Self::Value, Self::Value, Self::Value) {
        todo!()
    }

    fn compile_to_bytecode(&self, _source: rusty_js_core::Source) -> Option<Vec<u8>> {
        None
    }

    fn run_bytecode(&self, bytes: &[u8]) -> Self::Value {
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
