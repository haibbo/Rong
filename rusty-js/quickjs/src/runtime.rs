use crate::qjs;
use std::ptr::NonNull;

pub struct JSRtInner(pub(crate) NonNull<qjs::JSRuntime>);

impl JSRtInner {
    pub fn new() -> Result<Self, String> {
        let rt_ptr = unsafe { qjs::JS_NewRuntime() };
        let rt = NonNull::new(rt_ptr).ok_or_else(|| String::from("Failed to create JSRuntime"))?;
        Ok(JSRtInner(rt))
    }

    fn as_ptr(&self) -> *mut qjs::JSRuntime {
        self.0.as_ptr()
    }
}

impl Drop for JSRtInner {
    fn drop(&mut self) {
        unsafe {
            qjs::JS_FreeRuntime(self.0.as_ptr());
        }
    }
}
