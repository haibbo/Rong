use crate::JSCValue;
use crate::jsc;
use rong_core::{JSArrayBufferOps, JSValueImpl};
use std::ptr;
use std::slice;

impl JSArrayBufferOps for JSCValue {
    /// Create an ArrayBuffer by copying existing data
    fn from_bytes(ctx: &Self::Context, bytes: &[u8]) -> Self {
        Self::from_vec(ctx, bytes.to_vec())
    }

    /// Create an ArrayBuffer from an existing Vec without copying (zero-copy)
    fn from_vec(ctx: &Self::Context, mut vec: Vec<u8>) -> Self {
        unsafe {
            let mut exception: jsc::JSValueRef = ptr::null_mut();

            let len = vec.len();
            let ptr = vec.as_mut_ptr();

            // Keep the Vec alive until JavaScriptCore calls our deallocator callback.
            let deallocator_context = Box::into_raw(Box::new(vec)) as *mut ::std::os::raw::c_void;

            let array_buffer = jsc::JSObjectMakeArrayBufferWithBytesNoCopy(
                ctx.to_raw(),
                ptr as *mut _,
                len,
                Some(deallocator_callback),
                deallocator_context,
                &mut exception,
            );

            if !exception.is_null() {
                // Clean up the context if creation fails.
                let _ = Box::from_raw(deallocator_context as *mut Vec<u8>);
                JSCValue::from_owned_raw(ctx.to_raw(), exception).with_exception()
            } else {
                JSCValue::from_owned_obj(ctx.to_raw(), array_buffer)
            }
        }
    }

    /// Get the byte length of the ArrayBuffer
    fn length(&self) -> usize {
        unsafe {
            let mut exception: jsc::JSValueRef = ptr::null_mut();
            let len =
                jsc::JSObjectGetArrayBufferByteLength(self.ctx, self.as_obj(), &mut exception);
            if !exception.is_null() {
                0
            } else {
                len as usize
            }
        }
    }

    /// Get a safe slice view of the ArrayBuffer's data
    fn as_slice(&self) -> &[u8] {
        unsafe {
            let mut exception: jsc::JSValueRef = ptr::null_mut();
            let ptr = jsc::JSObjectGetArrayBufferBytesPtr(self.ctx, self.as_obj(), &mut exception);

            if !exception.is_null() || ptr.is_null() {
                // Return empty slice if there's an error
                &[]
            } else {
                let len = self.length();
                slice::from_raw_parts(ptr as *const u8, len)
            }
        }
    }

    /// Get a mutable slice view of the ArrayBuffer's data
    fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe {
            let mut exception: jsc::JSValueRef = ptr::null_mut();
            let ptr = jsc::JSObjectGetArrayBufferBytesPtr(self.ctx, self.as_obj(), &mut exception);

            if !exception.is_null() || ptr.is_null() {
                // Return empty slice if there's an error
                &mut []
            } else {
                let len = self.length();
                slice::from_raw_parts_mut(ptr as *mut u8, len)
            }
        }
    }
}

// Callback for deallocating ArrayBuffer memory
unsafe extern "C" fn deallocator_callback(
    _bytes: *mut ::std::os::raw::c_void,
    deallocator_context: *mut ::std::os::raw::c_void,
) {
    if !deallocator_context.is_null() {
        // Drop the Vec<u8> we stored as the deallocator context.
        let _ = unsafe { Box::from_raw(deallocator_context as *mut Vec<u8>) };
    }
}
