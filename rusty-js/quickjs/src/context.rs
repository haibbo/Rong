use crate::qjs;
use crate::runtime::JSRtInner;
use std::ptr::NonNull;

pub struct JSCtxInner {
    ctx: NonNull<qjs::JSContext>,
}

impl JSCtxInner {
    pub fn new(rt: &JSRtInner) -> Result<Self, String> {
        let ctx_ptr = unsafe { qjs::JS_NewContext(rt.0.as_ptr()) };
        let ctx =
            NonNull::new(ctx_ptr).ok_or_else(|| String::from("Failed to create JSContext"))?;
        Ok(Self { ctx })
    }

    pub fn as_ptr(&self) -> *mut qjs::JSContext {
        self.ctx.as_ptr()
    }

    pub fn from_ffi(ctx: *mut qjs::JSContext) -> Self {
        let ctx = unsafe {
            qjs::JS_DupContext(ctx);
            NonNull::new_unchecked(ctx)
        };
        JSCtxInner { ctx }
    }
}

impl Drop for JSCtxInner {
    fn drop(&mut self) {
        unsafe {
            qjs::JS_FreeContext(self.ctx.as_ptr());
        }
    }
}

impl Clone for JSCtxInner {
    fn clone(&self) -> Self {
        unsafe {
            let ctx = qjs::JS_DupContext(self.ctx.as_ptr());
            Self::from_ffi(ctx)
        }
    }
}
