use crate::{qjs, QJSRuntime};
use rusty_js_core::{JSContextRaw, JSRuntime};

pub struct QJSContext {
    raw: *mut qjs::JSContext,
}

impl Drop for QJSContext {
    fn drop(&mut self) {
        // println!("free QJS Ctx");
        unsafe {
            qjs::JS_FreeContext(self.raw);
        }
    }
}

impl Clone for QJSContext {
    fn clone(&self) -> Self {
        Self {
            raw: unsafe { qjs::JS_DupContext(self.raw) },
        }
    }
}

impl QJSContext {
    pub fn from_ffi(raw: *mut qjs::JSContext) -> Self {
        Self {
            raw: unsafe { qjs::JS_DupContext(raw) },
        }
    }
}

impl JSContextRaw for QJSContext {
    type Raw = *mut qjs::JSContext;
    type Runtime = QJSRuntime;

    fn new(runtime: &JSRuntime<Self::Runtime>) -> Self {
        unsafe {
            Self {
                raw: qjs::JS_NewContext(*runtime.as_raw()),
            }
        }
    }
    fn as_raw(&self) -> &Self::Raw {
        &self.raw
    }
}
