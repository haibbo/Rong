use crate::QJSValue;
use crate::qjs;
use rong_core::{JSArrayBufferOps, JSValueImpl};
use std::slice;

impl JSArrayBufferOps for QJSValue {
    fn from_bytes(ctx: &Self::Context, bytes: &[u8]) -> Self {
        unsafe {
            let array_buffer = qjs::JS_NewArrayBufferCopy(
                ctx.to_raw(),
                bytes.as_ptr() as *const _,
                bytes.len() as _,
            );

            if qjs::QJS_IsException(ctx.to_raw(), array_buffer) {
                let exception = qjs::JS_GetException(ctx.to_raw());
                QJSValue::from_owned_raw(ctx.to_raw(), exception).with_exception()
            } else {
                QJSValue::from_owned_raw(ctx.to_raw(), array_buffer)
            }
        }
    }

    fn from_vec(ctx: &Self::Context, vec: Vec<u8>) -> Self {
        unsafe {
            let mut vec = vec;
            let len = vec.len();
            let data_ptr = vec.as_mut_ptr();
            let opaque = Box::into_raw(Box::new(vec)) as *mut ::std::os::raw::c_void;

            let array_buffer = qjs::JS_NewArrayBuffer(
                ctx.to_raw(),
                data_ptr as *mut _,
                len as _,
                Some(deallocator_callback),
                opaque,
                false, // is_shared = false
            );

            if qjs::QJS_IsException(ctx.to_raw(), array_buffer) {
                // Clean up the memory if creation fails
                let _ = Box::from_raw(opaque as *mut Vec<u8>);
                let exception = qjs::JS_GetException(ctx.to_raw());
                QJSValue::from_owned_raw(ctx.to_raw(), exception).with_exception()
            } else {
                QJSValue::from_owned_raw(ctx.to_raw(), array_buffer)
            }
        }
    }

    fn length(&self) -> usize {
        unsafe {
            let mut len: usize = 0;
            let ptr = qjs::JS_GetArrayBuffer(self.ctx, &mut len as *mut _, self.value);
            if !ptr.is_null() { len } else { 0 }
        }
    }

    fn as_slice(&self) -> &[u8] {
        unsafe {
            let mut len: usize = 0;
            let ptr = qjs::JS_GetArrayBuffer(self.ctx, &mut len as *mut _, self.value);
            if !ptr.is_null() {
                slice::from_raw_parts(ptr as *const u8, len)
            } else {
                &[]
            }
        }
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe {
            let mut len: usize = 0;
            let ptr = qjs::JS_GetArrayBuffer(self.ctx, &mut len as *mut _, self.value);
            if !ptr.is_null() {
                slice::from_raw_parts_mut(ptr, len)
            } else {
                &mut []
            }
        }
    }
}

// Callback for deallocating ArrayBuffer memory
unsafe extern "C" fn deallocator_callback(
    _rt: *mut qjs::JSRuntime,
    opaque: *mut ::std::os::raw::c_void,
    _ptr: *mut ::std::os::raw::c_void,
) {
    if !opaque.is_null() {
        // SAFETY: `opaque` was created by `Box::into_raw(Box::new(Vec<u8>))` in `from_vec`.
        // QuickJS calls this deallocator exactly once for the ArrayBuffer.
        unsafe {
            drop(Box::from_raw(opaque as *mut Vec<u8>));
        }
    }
}
