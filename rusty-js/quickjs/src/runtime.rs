use crate::{qjs, QJSContext, QJSValue};
use rusty_js_core::{JSEngine, JSRuntimeImpl};

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
        let rt = unsafe { qjs::JS_NewRuntime() };
        #[cfg(debug_assertions)]
        unsafe {
            if std::env::var("DUMPFLAGS").is_ok() {
                //0x200: dump every object free
                //0x4000: dump leaked objects and strings in JS_FreeRuntime
                //more flags, pls refer to quickjs.c
                qjs::JS_SetDumpFlags(rt, 0x200 | 0x4000);
            }
        }
        Self { rt }
    }

    fn to_ffi(&self) -> Self::FfiRuntime {
        self.rt
    }

    fn run_pending_jobs(&self) {
        unsafe {
            let mut ctx = std::ptr::null_mut();
            while qjs::JS_IsJobPending(self.rt) != 0 {
                qjs::JS_ExecutePendingJob(self.rt, &mut ctx);
            }
        }
    }
}

pub struct QuickJS;

impl JSEngine for QuickJS {
    type Value = QJSValue;
    type Context = QJSContext;
    type Runtime = QJSRuntime;

    fn name() -> &'static str {
        "QuickJS-NG"
    }

    fn version() -> String {
        unsafe {
            let c_str = qjs::JS_GetVersion();
            std::ffi::CStr::from_ptr(c_str)
                .to_str()
                .map(|s| s.to_string())
                .unwrap()
        }
    }
}
