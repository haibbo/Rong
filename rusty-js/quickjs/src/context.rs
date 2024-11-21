use crate::qjs;
use crate::runtime::JSRtInner;
use std::ops::Deref;
use std::ptr::NonNull;

/// JSCtxInner has ownership of JSContext, but JSCtxRef just reference
/// JSCtxRef is born for callback scenario
///
/// let ctx_ref=JSCtxRef::from_ffi(ctx);
/// some_func(ctx_ref, ...)
///
/// fn some_func(ctx &JSCtxInner) {}

pub struct JSCtxInner {
    ctx: NonNull<qjs::JSContext>,
}

pub(crate) struct JSCtxRef {
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
        JSCtxInner {
            ctx: NonNull::new(ctx).unwrap(),
        }
    }
}

impl JSCtxRef {
    pub fn from_ffi(ctx: *mut qjs::JSContext) -> Self {
        JSCtxRef {
            ctx: unsafe { NonNull::new_unchecked(ctx) },
        }
    }

    pub fn as_ptr(&self) -> *mut qjs::JSContext {
        self.ctx.as_ptr()
    }
}

impl Deref for JSCtxRef {
    type Target = JSCtxInner;

    fn deref(&self) -> &Self::Target {
        unsafe { std::mem::transmute(self) }
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
