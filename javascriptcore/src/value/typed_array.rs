use crate::jsc;
use crate::JSCValue;
use rong_js_core::{JSExceptionHandler, JSTypedArrayKind, JSTypedArrayOps, JSValueImpl};
use std::ptr;

impl JSTypedArrayOps for JSCValue {
    fn from_array_buffer(
        ctx: &Self::Context,
        kind: JSTypedArrayKind,
        buffer: Self,
        byte_offset: usize,
        length: Option<usize>,
    ) -> Self {
        unsafe {
            let mut exception: jsc::JSValueRef = ptr::null_mut();

            // Calculate element size based on type
            let element_size = match kind {
                JSTypedArrayKind::Int8 | JSTypedArrayKind::Uint8 => 1,
                JSTypedArrayKind::Int16 | JSTypedArrayKind::Uint16 => 2,
                JSTypedArrayKind::Int32 | JSTypedArrayKind::Uint32 | JSTypedArrayKind::Float32 => 4,
                JSTypedArrayKind::BigInt64
                | JSTypedArrayKind::BigUint64
                | JSTypedArrayKind::Float64 => 8,
            };

            // Get the buffer size and calculate the length if not provided
            let buffer_size = jsc::JSObjectGetArrayBufferByteLength(
                ctx.to_raw(),
                buffer.as_obj(),
                &mut exception,
            );
            if !exception.is_null() {
                return JSCValue::from_owned_raw(ctx.to_raw(), exception).with_exception();
            }

            // Validate byte_offset alignment
            if byte_offset % element_size != 0 {
                return ctx.throw_error("byte_offset must be aligned");
            }

            // Calculate available length
            let available_bytes = if buffer_size >= byte_offset {
                buffer_size - byte_offset
            } else {
                return ctx.throw_error("byte_offset out of range");
            };

            let max_length = available_bytes / element_size;
            let length = length.unwrap_or(max_length).min(max_length);

            // Map kind to JSTypedArrayType
            let array_type = match kind {
                JSTypedArrayKind::Int8 => jsc::JSTypedArrayType_kJSTypedArrayTypeInt8Array,
                JSTypedArrayKind::Int16 => jsc::JSTypedArrayType_kJSTypedArrayTypeInt16Array,
                JSTypedArrayKind::Int32 => jsc::JSTypedArrayType_kJSTypedArrayTypeInt32Array,
                JSTypedArrayKind::Uint8 => jsc::JSTypedArrayType_kJSTypedArrayTypeUint8Array,
                JSTypedArrayKind::Uint16 => jsc::JSTypedArrayType_kJSTypedArrayTypeUint16Array,
                JSTypedArrayKind::Uint32 => jsc::JSTypedArrayType_kJSTypedArrayTypeUint32Array,
                JSTypedArrayKind::Float32 => jsc::JSTypedArrayType_kJSTypedArrayTypeFloat32Array,
                JSTypedArrayKind::Float64 => jsc::JSTypedArrayType_kJSTypedArrayTypeFloat64Array,
                JSTypedArrayKind::BigInt64 => jsc::JSTypedArrayType_kJSTypedArrayTypeBigInt64Array,
                JSTypedArrayKind::BigUint64 => {
                    jsc::JSTypedArrayType_kJSTypedArrayTypeBigUint64Array
                }
            };

            // Create TypedArray with buffer and offset
            let obj = jsc::JSObjectMakeTypedArrayWithArrayBufferAndOffset(
                ctx.to_raw(),
                array_type,
                buffer.as_obj(),
                byte_offset,
                length,
                &mut exception,
            );

            if !exception.is_null() {
                JSCValue::from_owned_raw(ctx.to_raw(), exception).with_exception()
            } else {
                JSCValue::from_owned_obj(ctx.to_raw(), obj)
            }
        }
    }

    fn get_kind(&self) -> Option<JSTypedArrayKind> {
        unsafe {
            let mut exception: jsc::JSValueRef = ptr::null_mut();
            let array_type = jsc::JSValueGetTypedArrayType(self.ctx, self.as_obj(), &mut exception);
            if !exception.is_null() {
                None
            } else {
                match array_type {
                    jsc::JSTypedArrayType_kJSTypedArrayTypeInt8Array => {
                        Some(JSTypedArrayKind::Int8)
                    }
                    jsc::JSTypedArrayType_kJSTypedArrayTypeInt16Array => {
                        Some(JSTypedArrayKind::Int16)
                    }
                    jsc::JSTypedArrayType_kJSTypedArrayTypeInt32Array => {
                        Some(JSTypedArrayKind::Int32)
                    }
                    jsc::JSTypedArrayType_kJSTypedArrayTypeUint8Array => {
                        Some(JSTypedArrayKind::Uint8)
                    }
                    jsc::JSTypedArrayType_kJSTypedArrayTypeUint16Array => {
                        Some(JSTypedArrayKind::Uint16)
                    }
                    jsc::JSTypedArrayType_kJSTypedArrayTypeUint32Array => {
                        Some(JSTypedArrayKind::Uint32)
                    }
                    jsc::JSTypedArrayType_kJSTypedArrayTypeFloat32Array => {
                        Some(JSTypedArrayKind::Float32)
                    }
                    jsc::JSTypedArrayType_kJSTypedArrayTypeFloat64Array => {
                        Some(JSTypedArrayKind::Float64)
                    }
                    jsc::JSTypedArrayType_kJSTypedArrayTypeBigInt64Array => {
                        Some(JSTypedArrayKind::BigInt64)
                    }
                    jsc::JSTypedArrayType_kJSTypedArrayTypeBigUint64Array => {
                        Some(JSTypedArrayKind::BigUint64)
                    }
                    _ => None,
                }
            }
        }
    }

    fn get_array_buffer(&self) -> Option<Self> {
        unsafe {
            let mut exception: jsc::JSValueRef = ptr::null_mut();
            let buffer = jsc::JSObjectGetTypedArrayBuffer(self.ctx, self.as_obj(), &mut exception);
            if !exception.is_null() {
                None
            } else {
                Some(JSCValue::from_owned_obj(self.ctx, buffer))
            }
        }
    }

    fn get_byte_offset(&self) -> usize {
        unsafe {
            let mut exception: jsc::JSValueRef = ptr::null_mut();
            let offset =
                jsc::JSObjectGetTypedArrayByteOffset(self.ctx, self.as_obj(), &mut exception);
            if !exception.is_null() {
                0
            } else {
                offset
            }
        }
    }

    fn get_length(&self) -> usize {
        unsafe {
            let mut exception: jsc::JSValueRef = ptr::null_mut();
            let length = jsc::JSObjectGetTypedArrayLength(self.ctx, self.as_obj(), &mut exception);
            if !exception.is_null() {
                0
            } else {
                length
            }
        }
    }

    fn get_byte_length(&self) -> usize {
        unsafe {
            let mut exception: jsc::JSValueRef = ptr::null_mut();
            let byte_length =
                jsc::JSObjectGetTypedArrayByteLength(self.ctx, self.as_obj(), &mut exception);
            if !exception.is_null() {
                0
            } else {
                byte_length
            }
        }
    }
}
