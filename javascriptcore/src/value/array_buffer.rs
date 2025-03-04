use crate::jsc;
use crate::JSCValue;
use rusty_js_core::{JSArrayBufferOps, JSValueImpl};
use std::ptr;
use std::slice;

impl JSArrayBufferOps for JSCValue {
    /// Create an ArrayBuffer by copying existing data
    fn from_bytes(ctx: &Self::Context, bytes: &[u8]) -> Self {
        unsafe {
            let mut exception: jsc::JSValueRef = ptr::null_mut();
            
            // Create a copy of the bytes
            let mut data = bytes.to_vec();
            let len = data.len();
            let ptr = data.as_mut_ptr();
            
            // Take ownership of the data to prevent it from being dropped
            std::mem::forget(data);

            let array_buffer = jsc::JSObjectMakeArrayBufferWithBytesNoCopy(
                ctx.to_raw(),
                ptr as *mut _,
                len,
                Some(deallocator_callback),
                ptr::null_mut(),
                &mut exception,
            );

            if !exception.is_null() {
                // Clean up the memory if creation fails
                let _ = Vec::from_raw_parts(ptr, len, len);
                JSCValue::from_owned_raw(ctx.to_raw(), exception).with_exception()
            } else {
                JSCValue::from_owned_obj(ctx.to_raw(), array_buffer)
            }
        }
    }

    /// Create an ArrayBuffer from an existing Vec without copying (zero-copy)
    fn from_vec(ctx: &Self::Context, mut vec: Vec<u8>) -> Self {
        unsafe {
            let mut exception: jsc::JSValueRef = ptr::null_mut();
            
            let len = vec.len();
            let ptr = vec.as_mut_ptr();
            
            // Take ownership of the vec to prevent it from being dropped
            std::mem::forget(vec);

            let array_buffer = jsc::JSObjectMakeArrayBufferWithBytesNoCopy(
                ctx.to_raw(),
                ptr as *mut _,
                len,
                Some(deallocator_callback),
                ptr::null_mut(),
                &mut exception,
            );

            if !exception.is_null() {
                // Clean up the memory if creation fails
                let _ = Vec::from_raw_parts(ptr, len, len);
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
            let len = jsc::JSObjectGetArrayBufferByteLength(self.ctx, self.as_obj(), &mut exception);
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
    bytes: *mut ::std::os::raw::c_void,
    _deallocator_context: *mut ::std::os::raw::c_void,
) {
    if !bytes.is_null() {
        // Get the length from the ArrayBuffer before deallocating
        let len = {
            let slice = slice::from_raw_parts(bytes as *const u8, 0);
            slice.as_ptr().align_offset(std::mem::align_of::<u8>())
        };
        
        // Reconstruct the Vec and let it drop
        let _ = Vec::from_raw_parts(bytes as *mut u8, len, len);
    }
}
