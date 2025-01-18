use crate::{jsc, JSCContext, JSCValue};
use rusty_js_core::{JSEngine, JSRuntimeImpl};

pub struct JSCRuntime {
    raw: jsc::JSContextGroupRef,
}

impl JSRuntimeImpl for JSCRuntime {
    type RawRuntime = jsc::JSContextGroupRef;
    type Context = JSCContext;

    fn new() -> Self {
        todo!()
    }

    fn to_raw(&self) -> Self::RawRuntime {
        todo!()
    }

    fn run_pending_jobs(&self) {
        todo!()
    }

    fn run_gc(&self) {
        todo!()
    }
}

pub struct JavaScriptCore;

impl JSEngine for JavaScriptCore {
    type Value = JSCValue;
    type Context = JSCContext;
    type Runtime = JSCRuntime;

    fn name() -> &'static str {
        "JavaScriptCore"
    }

    fn version() -> String {
        String::from("Unkown")
    }
}
