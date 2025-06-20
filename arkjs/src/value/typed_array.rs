use crate::{ArkJSValue, arkjs};
use rong_core::{JSTypedArrayKind, JSTypedArrayOps, JSValueImpl};
use std::ptr;

impl JSTypedArrayOps for ArkJSValue {
    fn from_array_buffer(
        ctx: &Self::Context,
        kind: JSTypedArrayKind,
        buffer: Self,
        byte_offset: usize,
        length: Option<usize>,
    ) -> Self {
        unsafe {
            let mut typed_array: arkjs::JSVM_Value = ptr::null_mut();

            // Convert JSTypedArrayKind to Ark JS type
            let ark_type = match kind {
                JSTypedArrayKind::Int8 => arkjs::JSVM_TypedarrayType_JSVM_INT8_ARRAY,
                JSTypedArrayKind::Uint8 => arkjs::JSVM_TypedarrayType_JSVM_UINT8_ARRAY,
                JSTypedArrayKind::Int16 => arkjs::JSVM_TypedarrayType_JSVM_INT16_ARRAY,
                JSTypedArrayKind::Uint16 => arkjs::JSVM_TypedarrayType_JSVM_UINT16_ARRAY,
                JSTypedArrayKind::Int32 => arkjs::JSVM_TypedarrayType_JSVM_INT32_ARRAY,
                JSTypedArrayKind::Uint32 => arkjs::JSVM_TypedarrayType_JSVM_UINT32_ARRAY,
                JSTypedArrayKind::Float32 => arkjs::JSVM_TypedarrayType_JSVM_FLOAT32_ARRAY,
                JSTypedArrayKind::Float64 => arkjs::JSVM_TypedarrayType_JSVM_FLOAT64_ARRAY,
                JSTypedArrayKind::BigInt64 => arkjs::JSVM_TypedarrayType_JSVM_BIGINT64_ARRAY,
                JSTypedArrayKind::BigUint64 => arkjs::JSVM_TypedarrayType_JSVM_BIGUINT64_ARRAY,
            };

            let actual_length = length.unwrap_or(0);
            let status = arkjs::OH_JSVM_CreateTypedarray(
                ctx.to_raw(),
                ark_type,
                actual_length,
                *buffer.as_raw_value(),
                byte_offset,
                &mut typed_array,
            );

            if status == arkjs::JSVM_Status_JSVM_OK {
                ArkJSValue::from_owned_raw(ctx.to_raw(), typed_array).with_object()
            } else {
                Self::create_undefined(ctx)
            }
        }
    }

    fn get_kind(&self) -> Option<JSTypedArrayKind> {
        unsafe {
            let mut array_type: arkjs::JSVM_TypedarrayType =
                arkjs::JSVM_TypedarrayType_JSVM_INT8_ARRAY;
            let mut length: usize = 0;
            let mut data: *mut std::ffi::c_void = ptr::null_mut();
            let mut array_buffer: arkjs::JSVM_Value = ptr::null_mut();
            let mut byte_offset: usize = 0;

            let status = arkjs::OH_JSVM_GetTypedarrayInfo(
                self.env,
                self.value,
                &mut array_type,
                &mut length,
                &mut data,
                &mut array_buffer,
                &mut byte_offset,
            );

            if status == arkjs::JSVM_Status_JSVM_OK {
                let kind = match array_type {
                    arkjs::JSVM_TypedarrayType_JSVM_INT8_ARRAY => JSTypedArrayKind::Int8,
                    arkjs::JSVM_TypedarrayType_JSVM_UINT8_ARRAY => JSTypedArrayKind::Uint8,
                    arkjs::JSVM_TypedarrayType_JSVM_UINT8_CLAMPED_ARRAY => JSTypedArrayKind::Uint8, // Map to Uint8
                    arkjs::JSVM_TypedarrayType_JSVM_INT16_ARRAY => JSTypedArrayKind::Int16,
                    arkjs::JSVM_TypedarrayType_JSVM_UINT16_ARRAY => JSTypedArrayKind::Uint16,
                    arkjs::JSVM_TypedarrayType_JSVM_INT32_ARRAY => JSTypedArrayKind::Int32,
                    arkjs::JSVM_TypedarrayType_JSVM_UINT32_ARRAY => JSTypedArrayKind::Uint32,
                    arkjs::JSVM_TypedarrayType_JSVM_FLOAT32_ARRAY => JSTypedArrayKind::Float32,
                    arkjs::JSVM_TypedarrayType_JSVM_FLOAT64_ARRAY => JSTypedArrayKind::Float64,
                    arkjs::JSVM_TypedarrayType_JSVM_BIGINT64_ARRAY => JSTypedArrayKind::BigInt64,
                    arkjs::JSVM_TypedarrayType_JSVM_BIGUINT64_ARRAY => JSTypedArrayKind::BigUint64,
                    _ => return None,
                };
                Some(kind)
            } else {
                None
            }
        }
    }

    fn get_array_buffer(&self) -> Option<Self> {
        unsafe {
            let mut array_type: arkjs::JSVM_TypedarrayType =
                arkjs::JSVM_TypedarrayType_JSVM_INT8_ARRAY;
            let mut length: usize = 0;
            let mut data: *mut std::ffi::c_void = ptr::null_mut();
            let mut array_buffer: arkjs::JSVM_Value = ptr::null_mut();
            let mut byte_offset: usize = 0;

            let status = arkjs::OH_JSVM_GetTypedarrayInfo(
                self.env,
                self.value,
                &mut array_type,
                &mut length,
                &mut data,
                &mut array_buffer,
                &mut byte_offset,
            );

            if status == arkjs::JSVM_Status_JSVM_OK && !array_buffer.is_null() {
                Some(ArkJSValue::from_owned_raw(self.env, array_buffer))
            } else {
                None
            }
        }
    }

    fn get_byte_offset(&self) -> usize {
        unsafe {
            let mut array_type: arkjs::JSVM_TypedarrayType =
                arkjs::JSVM_TypedarrayType_JSVM_INT8_ARRAY;
            let mut length: usize = 0;
            let mut data: *mut std::ffi::c_void = ptr::null_mut();
            let mut array_buffer: arkjs::JSVM_Value = ptr::null_mut();
            let mut byte_offset: usize = 0;

            let status = arkjs::OH_JSVM_GetTypedarrayInfo(
                self.env,
                self.value,
                &mut array_type,
                &mut length,
                &mut data,
                &mut array_buffer,
                &mut byte_offset,
            );

            if status == arkjs::JSVM_Status_JSVM_OK {
                byte_offset
            } else {
                0
            }
        }
    }

    fn get_length(&self) -> usize {
        unsafe {
            let mut array_type: arkjs::JSVM_TypedarrayType =
                arkjs::JSVM_TypedarrayType_JSVM_INT8_ARRAY;
            let mut length: usize = 0;
            let mut data: *mut std::ffi::c_void = ptr::null_mut();
            let mut array_buffer: arkjs::JSVM_Value = ptr::null_mut();
            let mut byte_offset: usize = 0;

            let status = arkjs::OH_JSVM_GetTypedarrayInfo(
                self.env,
                self.value,
                &mut array_type,
                &mut length,
                &mut data,
                &mut array_buffer,
                &mut byte_offset,
            );

            if status == arkjs::JSVM_Status_JSVM_OK {
                length
            } else {
                0
            }
        }
    }

    fn get_byte_length(&self) -> usize {
        let length = self.get_length();
        let element_size = match self.get_kind() {
            Some(JSTypedArrayKind::Int8) | Some(JSTypedArrayKind::Uint8) => 1,
            Some(JSTypedArrayKind::Int16) | Some(JSTypedArrayKind::Uint16) => 2,
            Some(JSTypedArrayKind::Int32)
            | Some(JSTypedArrayKind::Uint32)
            | Some(JSTypedArrayKind::Float32) => 4,
            Some(JSTypedArrayKind::BigInt64)
            | Some(JSTypedArrayKind::BigUint64)
            | Some(JSTypedArrayKind::Float64) => 8,
            None => 0,
        };
        length * element_size
    }
}
