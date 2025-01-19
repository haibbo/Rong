use crate::{jsc, JSCRuntime, JSCValue};
use rusty_js_core::{JSContextImpl, JSRuntimeImpl};

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

    fn get_opaque<T>(ctx: &Self::RawContext) -> *mut T {
        std::ptr::null_mut()
    }

    fn set_opaque<T>(ctx: &Self::RawContext, opaque: *mut T) {
        //    todo!()
    }

    fn as_raw(&self) -> &Self::RawContext {
        &self.raw
    }

    fn from_borrowed_raw(ctx: Self::RawContext) -> Self {
        todo!()
    }

    fn eval(&self, source: rusty_js_core::Source) -> Self::Value {
        todo!()
    }

    fn global(&self) -> Self::Value {
        todo!()
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
