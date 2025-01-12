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

        //0x200: dump every object free
        //0x4000: dump leaked objects and strings in JS_FreeRuntime
        //more flags, pls refer to quickjs.c
        #[cfg(debug_assertions)]
        if let Ok(flags) = std::env::var("DUMPFLAGS") {
            let flags = if flags.starts_with("0x") {
                u64::from_str_radix(flags.trim_start_matches("0x"), 16).unwrap_or(0x4000 | 0x200)
            } else {
                0x4000 | 0x200
            };
            println!("Dump flags: 0x{:x}", flags);
            unsafe {
                qjs::JS_SetDumpFlags(rt, flags);
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
