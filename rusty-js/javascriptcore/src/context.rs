use crate::{jsc, JSCRuntime, JSCValue};
use rusty_js_core::JSContextImpl;

pub struct JSCContext {
    raw: jsc::JSContextRef,
}

impl JSContextImpl for JSCContext {
    type RawContext = jsc::JSContextRef;
    type Runtime = JSCRuntime;
    type Value = JSCValue;

    fn new(runtime: &Self::Runtime) -> Self {
        todo!()
    }

    fn get_opaque<T>(ctx: &Self::RawContext) -> *mut T {
        todo!()
    }

    fn set_opaque<T>(ctx: &Self::RawContext, opaque: *mut T) {
        todo!()
    }

    fn as_raw(&self) -> &Self::RawContext {
        todo!()
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
