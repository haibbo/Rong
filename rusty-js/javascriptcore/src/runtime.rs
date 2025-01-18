use crate::{jsc, JSCContext, JSCValue};
use rusty_js_core::{JSEngine, JSRuntimeImpl};

pub struct JSCRuntime {
    raw: *const jsc::OpaqueJSContextGroup,
}

impl JSRuntimeImpl for JSCRuntime {
    type RawRuntime = *const jsc::OpaqueJSContextGroup;
    type Context = JSCContext;

    fn new() -> Self {
        Self {
            raw: unsafe { jsc::JSContextGroupCreate() },
        }
    }

    fn to_raw(&self) -> Self::RawRuntime {
        self.raw
    }

    // JavaScriptCore has no this API
    fn run_pending_jobs(&self) {}

    // JavaScriptCore has no this API
    fn run_gc(&self) {}
}

impl Drop for JSCRuntime {
    fn drop(&mut self) {
        unsafe {
            jsc::JSContextGroupRelease(self.raw);
        }
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
