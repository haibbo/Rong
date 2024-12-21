use crate::{qjs, QJSContext, QJSValue};
use rusty_js_core::{JSContextImpl, JSEngine, JSRuntimeImpl};

pub struct QJSRuntime {
    rt: *mut qjs::JSRuntime,
}

impl Drop for QJSRuntime {
    fn drop(&mut self) {
        // println!("free QJS Runtime");
        unsafe {
            qjs::JS_FreeRuntime(self.rt);
        }
    }
}

impl JSRuntimeImpl for QJSRuntime {
    type FfiRuntime = *mut qjs::JSRuntime;
    type Context = QJSContext;

    // new QuickJS JS Runtime
    fn new() -> Self {
        Self {
            rt: unsafe { qjs::JS_NewRuntime() },
        }
    }

    fn to_ffi(&self) -> Self::FfiRuntime {
        self.rt
    }
}

pub struct QuickJS;

impl JSEngine for QuickJS {
    type Value = QJSValue;
    type Context = QJSContext;
    type Runtime = QJSRuntime;

    fn _runtime() -> Self::Runtime {
        QJSRuntime::new()
    }
    fn _context(rt: &Self::Runtime) -> Self::Context {
        QJSContext::new(rt)
    }
}
