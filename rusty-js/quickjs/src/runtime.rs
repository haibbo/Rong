use crate::{qjs, QJSContext};
use rusty_js_core::JSRuntimeImpl;

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
