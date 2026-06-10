use crate::{JSCContext, JSCValue, jsc};
use rong_core::{JSEngine, JSRuntimeImpl};

pub struct JSCRuntime {
    raw: *const jsc::OpaqueJSContextGroup,
}

impl JSRuntimeImpl for JSCRuntime {
    type RawRuntime = *const jsc::OpaqueJSContextGroup;
    type Context = JSCContext;

    fn new() -> Self {
        // On the source/JSCOnly backend, force JSC's one-time global init before
        // the first JSC API call. `JSContextGroupCreate` is the earliest VM touch
        // (earlier than `JSCContext::new`), so the guard belongs here. Idempotent
        // and thread-safe; a no-op on the system framework, which runs this from
        // the dylib's static initializers.
        #[cfg(jsc_source)]
        jsc::ensure_initialized();
        Self {
            raw: unsafe { jsc::JSContextGroupCreate() },
        }
    }

    fn to_raw(&self) -> Self::RawRuntime {
        self.raw
    }

    // JavaScriptCore  GC works on Conext level, not runtime
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
        #[cfg(jsc_source)]
        {
            option_env!("RONG_JSC_WEBKIT_REVISION")
                .map(|revision| format!("source:{revision}"))
                .unwrap_or_else(|| String::from("source"))
        }
        #[cfg(not(jsc_source))]
        {
            String::from("framework")
        }
    }
}
