use crate::{qjs, QJSContext};
use rusty_js_core::JSRuntimeImpl;

pub struct QJSRuntime {
    raw: *mut qjs::JSRuntime,
}

impl Drop for QJSRuntime {
    fn drop(&mut self) {
        // println!("free QJS Runtime");
        unsafe {
            qjs::JS_FreeRuntime(self.raw);
        }
    }
}

impl JSRuntimeImpl for QJSRuntime {
    type RawRuntime = *mut qjs::JSRuntime;
    type Context = QJSContext;

    // new raw JS Runtime
    fn new() -> Self {
        Self {
            raw: unsafe { qjs::JS_NewRuntime() },
        }
    }

    fn as_raw(&self) -> &Self::RawRuntime {
        &self.raw
    }
}
