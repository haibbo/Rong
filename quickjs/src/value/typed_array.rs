use crate::QJSValue;
use crate::qjs;
use rong_core::{JSTypedArrayKind, JSTypedArrayOps, JSValueImpl};

impl JSTypedArrayOps for QJSValue {
    fn from_array_buffer(
        ctx: &Self::Context,
        kind: JSTypedArrayKind,
        buffer: Self,
        byte_offset: usize,
        length: Option<usize>,
    ) -> Self {
        let ctx = ctx.to_raw();
        unsafe {
            let array_type = match kind {
                JSTypedArrayKind::Int8 => qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_INT8,
                JSTypedArrayKind::Uint8 => qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8,
                JSTypedArrayKind::Uint8Clamped => qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8C,
                JSTypedArrayKind::Int16 => qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_INT16,
                JSTypedArrayKind::Uint16 => qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT16,
                JSTypedArrayKind::Int32 => qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_INT32,
                JSTypedArrayKind::Uint32 => qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT32,
                JSTypedArrayKind::BigInt64 => qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_BIG_INT64,
                JSTypedArrayKind::BigUint64 => qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_BIG_UINT64,
                JSTypedArrayKind::Float32 => qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_FLOAT32,
                JSTypedArrayKind::Float64 => qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_FLOAT64,
            };

            // Create arguments for the new typed array
            //
            // Important:
            // quickjs JS_NewTypedArray does not support u64 as offset_val
            let offset_val = qjs::QJS_NewUint32(ctx, byte_offset as u32);
            let length_val = match length {
                Some(len) => qjs::QJS_NewUint32(ctx, len as u32),
                None => qjs::QJS_NewUndefined(ctx),
            };

            // Create the typed array
            let mut args = [buffer.value, offset_val, length_val];
            let array = qjs::JS_NewTypedArray(ctx, 3, args.as_mut_ptr(), array_type);

            if qjs::QJS_IsException(ctx, array) {
                let exception = qjs::JS_GetException(ctx);
                QJSValue::from_owned_raw(ctx, exception).with_exception()
            } else {
                QJSValue::from_owned_raw(ctx, array)
            }
        }
    }

    fn get_kind(&self) -> Option<JSTypedArrayKind> {
        unsafe {
            // QuickJS returns -1 for non-typed arrays, so keep the raw signed value.
            let array_type = qjs::JS_GetTypedArrayType(self.value);

            match array_type {
                x if x == qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8C as i32 => {
                    Some(JSTypedArrayKind::Uint8Clamped)
                }
                x if x == qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_INT8 as i32 => {
                    Some(JSTypedArrayKind::Int8)
                }
                x if x == qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT8 as i32 => {
                    Some(JSTypedArrayKind::Uint8)
                }
                x if x == qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_INT16 as i32 => {
                    Some(JSTypedArrayKind::Int16)
                }
                x if x == qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT16 as i32 => {
                    Some(JSTypedArrayKind::Uint16)
                }
                x if x == qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_INT32 as i32 => {
                    Some(JSTypedArrayKind::Int32)
                }
                x if x == qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_UINT32 as i32 => {
                    Some(JSTypedArrayKind::Uint32)
                }
                x if x == qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_BIG_INT64 as i32 => {
                    Some(JSTypedArrayKind::BigInt64)
                }
                x if x == qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_BIG_UINT64 as i32 => {
                    Some(JSTypedArrayKind::BigUint64)
                }
                x if x == qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_FLOAT32 as i32 => {
                    Some(JSTypedArrayKind::Float32)
                }
                x if x == qjs::JSTypedArrayEnum_JS_TYPED_ARRAY_FLOAT64 as i32 => {
                    Some(JSTypedArrayKind::Float64)
                }
                _ => None,
            }
        }
    }

    fn get_array_buffer(&self) -> Option<Self> {
        unsafe {
            let mut pbyte_length: usize = 0;
            let mut pbyte_offset: usize = 0;
            let mut pbytes_per_element: usize = 0;
            let buffer = qjs::JS_GetTypedArrayBuffer(
                self.ctx,
                self.value,
                &mut pbyte_offset as *mut _,
                &mut pbyte_length as *mut _,
                &mut pbytes_per_element as *mut _,
            );
            if qjs::QJS_IsException(self.ctx, buffer) {
                None
            } else {
                Some(QJSValue::from_owned_raw(self.ctx, buffer))
            }
        }
    }

    fn get_byte_offset(&self) -> usize {
        unsafe {
            let mut pbyte_length: usize = 0;
            let mut pbyte_offset: usize = 0;
            let mut pbytes_per_element: usize = 0;

            let buffer = qjs::JS_GetTypedArrayBuffer(
                self.ctx,
                self.value,
                &mut pbyte_offset as *mut _,
                &mut pbyte_length as *mut _,
                &mut pbytes_per_element as *mut _,
            );
            qjs::JS_FreeValue(self.ctx, buffer);

            pbyte_offset
        }
    }

    fn get_length(&self) -> usize {
        unsafe {
            let mut pbyte_length: usize = 0;
            let mut pbyte_offset: usize = 0;
            let mut pbytes_per_element: usize = 0;

            let buffer = qjs::JS_GetTypedArrayBuffer(
                self.ctx,
                self.value,
                &mut pbyte_offset as *mut _,
                &mut pbyte_length as *mut _,
                &mut pbytes_per_element as *mut _,
            );
            qjs::JS_FreeValue(self.ctx, buffer);

            if pbytes_per_element == 0 {
                0
            } else {
                pbyte_length / pbytes_per_element
            }
        }
    }

    fn get_byte_length(&self) -> usize {
        unsafe {
            let mut pbyte_length: usize = 0;
            let mut pbyte_offset: usize = 0;
            let mut pbytes_per_element: usize = 0;

            let buffer = qjs::JS_GetTypedArrayBuffer(
                self.ctx,
                self.value,
                &mut pbyte_offset as *mut _,
                &mut pbyte_length as *mut _,
                &mut pbytes_per_element as *mut _,
            );
            qjs::JS_FreeValue(self.ctx, buffer);

            pbyte_length
        }
    }
}
