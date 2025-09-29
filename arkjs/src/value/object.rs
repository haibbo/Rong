use crate::{ArkJSValue, arkjs};
use rong_core::{JSObjectOps, JSValueImpl, PropertyAttributes};

impl JSObjectOps for ArkJSValue {
    fn new_object(ctx: &Self::Context) -> Self {
        unsafe {
            let mut obj: arkjs::JSVM_Value = std::ptr::null_mut();
            let status = arkjs::OH_JSVM_CreateObject(ctx.to_raw(), &mut obj);
            if status == arkjs::JSVM_Status_JSVM_OK {
                ArkJSValue::from_owned_raw(ctx.to_raw(), obj).with_object()
            } else {
                Self::create_undefined(ctx)
            }
        }
    }

    fn make_instance(ctx: &Self::Context, constructor: Self, data: *mut ()) -> Self {
        unsafe {
            let mut instance: arkjs::JSVM_Value = std::ptr::null_mut();

            // Create a new instance using the constructor
            let status = arkjs::OH_JSVM_NewInstance(
                ctx.to_raw(),
                *constructor.as_raw_value(),
                0,
                std::ptr::null(),
                &mut instance,
            );

            if status == arkjs::JSVM_Status_JSVM_OK {
                // Set the native data if provided
                if !data.is_null() {
                    let mut ref_value: arkjs::JSVM_Ref = std::ptr::null_mut();

                    // Get the correct finalizer function for this constructor
                    let finalizer_fn =
                        crate::class::get_finalizer_by_constructor(constructor.clone());

                    // Pass the JS instance value directly as finalizeHint
                    // Since JSVM_Value is typedef struct JSVM_Value__*, we can cast it directly
                    let finalize_hint = instance as *mut std::ffi::c_void;

                    arkjs::OH_JSVM_Wrap(
                        ctx.to_raw(),
                        instance,
                        data as *mut std::ffi::c_void,
                        finalizer_fn,
                        finalize_hint,
                        &mut ref_value,
                    );
                }
                ArkJSValue::from_owned_raw(ctx.to_raw(), instance).with_object()
            } else {
                Self::create_undefined(ctx)
            }
        }
    }

    fn get_opaque(&self) -> *mut () {
        unsafe {
            let mut data: *mut std::ffi::c_void = std::ptr::null_mut();
            let status = arkjs::OH_JSVM_Unwrap(self.env, self.value, &mut data);

            if status == arkjs::JSVM_Status_JSVM_OK {
                data as *mut ()
            } else {
                std::ptr::null_mut()
            }
        }
    }

    fn del_property(&self, key: Self) -> bool {
        unsafe {
            let mut result = false;
            let status = arkjs::OH_JSVM_DeleteProperty(
                self.env,
                self.value,
                *key.as_raw_value(),
                &mut result,
            );
            status == arkjs::JSVM_Status_JSVM_OK && result
        }
    }

    fn has_property(&self, key: Self) -> bool {
        unsafe {
            let mut has_property = false;
            let status = arkjs::OH_JSVM_HasProperty(
                self.env,
                self.value,
                *key.as_raw_value(),
                &mut has_property,
            );
            status == arkjs::JSVM_Status_JSVM_OK && has_property
        }
    }

    fn set_property(&self, key: Self, value: Self) -> bool {
        unsafe {
            let status = arkjs::OH_JSVM_SetProperty(
                self.env,
                self.value,
                *key.as_raw_value(),
                *value.as_raw_value(),
            );
            status == arkjs::JSVM_Status_JSVM_OK
        }
    }

    fn set_prototype(&self, prototype: Self) -> bool {
        unsafe {
            // Note: ArkJS might not have OH_JSVM_SetPrototype, using alternative approach
            let mut proto_key: arkjs::JSVM_Value = std::ptr::null_mut();
            let status = arkjs::OH_JSVM_CreateStringUtf8(
                self.env,
                b"__proto__\0".as_ptr() as *const u8,
                9,
                &mut proto_key,
            );

            if status == arkjs::JSVM_Status_JSVM_OK {
                let status = arkjs::OH_JSVM_SetProperty(
                    self.env,
                    self.value,
                    proto_key,
                    *prototype.as_raw_value(),
                );
                status == arkjs::JSVM_Status_JSVM_OK
            } else {
                false
            }
        }
    }

    fn define_property(
        &self,
        key: Self,
        value: Self,
        _getter: Self,
        _setter: Self,
        attributes: PropertyAttributes,
    ) -> bool {
        unsafe {
            let descriptor = arkjs::JSVM_PropertyDescriptor {
                utf8name: std::ptr::null(),
                name: *key.as_raw_value(),
                method: std::ptr::null_mut(),
                getter: std::ptr::null_mut(),
                setter: std::ptr::null_mut(),
                value: if attributes.has_value() {
                    *value.as_raw_value()
                } else {
                    std::ptr::null_mut()
                },
                attributes: to_arkjs_attributes(attributes),
            };

            let status = arkjs::OH_JSVM_DefineProperties(self.env, self.value, 1, &descriptor);
            status == arkjs::JSVM_Status_JSVM_OK
        }
    }

    fn get_property(&self, key: Self) -> Option<Self> {
        unsafe {
            let mut result: arkjs::JSVM_Value = std::ptr::null_mut();
            let status =
                arkjs::OH_JSVM_GetProperty(self.env, self.value, *key.as_raw_value(), &mut result);

            if status == arkjs::JSVM_Status_JSVM_OK && !result.is_null() {
                Some(ArkJSValue::from_owned_raw(self.env, result))
            } else {
                None
            }
        }
    }

    fn instance_of(&self, constructor: Self) -> bool {
        unsafe {
            let mut result = false;
            let status = arkjs::OH_JSVM_Instanceof(
                self.env,
                self.value,
                *constructor.as_raw_value(),
                &mut result,
            );
            status == arkjs::JSVM_Status_JSVM_OK && result
        }
    }

    fn get_own_property_names(&self) -> Option<Vec<Self>> {
        unsafe {
            let mut property_names: arkjs::JSVM_Value = std::ptr::null_mut();
            let status = arkjs::OH_JSVM_GetPropertyNames(self.env, self.value, &mut property_names);

            if status != arkjs::JSVM_Status_JSVM_OK {
                return None;
            }

            // Get array length
            let mut length: u32 = 0;
            let status = arkjs::OH_JSVM_GetArrayLength(self.env, property_names, &mut length);

            if status != arkjs::JSVM_Status_JSVM_OK {
                return None;
            }

            let mut properties = Vec::with_capacity(length as usize);

            for i in 0..length {
                let mut element: arkjs::JSVM_Value = std::ptr::null_mut();
                let status = arkjs::OH_JSVM_GetElement(self.env, property_names, i, &mut element);

                if status == arkjs::JSVM_Status_JSVM_OK {
                    properties.push(ArkJSValue::from_owned_raw(self.env, element));
                }
            }

            Some(properties)
        }
    }
}

// Helper function to convert PropertyAttributes to Ark JS attributes
fn to_arkjs_attributes(attr: PropertyAttributes) -> arkjs::JSVM_PropertyAttributes {
    let mut flags = arkjs::JSVM_PropertyAttributes_JSVM_DEFAULT;

    if attr.is_writable() {
        flags |= arkjs::JSVM_PropertyAttributes_JSVM_WRITABLE;
    }
    if attr.is_enumerable() {
        flags |= arkjs::JSVM_PropertyAttributes_JSVM_ENUMERABLE;
    }
    if attr.is_configurable() {
        flags |= arkjs::JSVM_PropertyAttributes_JSVM_CONFIGURABLE;
    }

    flags
}
