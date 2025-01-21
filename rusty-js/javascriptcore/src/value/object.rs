use crate::{jsc, JSCValue};
use rusty_js_core::{JSObjectOps, JSValueImpl, PropertyAttributes};

impl JSObjectOps for JSCValue {
    fn new_object(ctx: &Self::Context) -> Self {
        unsafe {
            let obj = jsc::JSObjectMake(ctx.to_raw(), std::ptr::null_mut(), std::ptr::null_mut());
            JSCValue::from_owned_raw(ctx.to_raw(), obj)
        }
    }

    fn make_object<T>(ctx: &Self::Context, constructor: Self, data: *mut T) -> Self {
        unsafe {
            let obj = jsc::JSObjectMake(ctx.to_raw(), constructor.value as _, data as *mut _);
            JSCValue::from_owned_raw(ctx.to_raw(), obj)
        }
    }

    fn get_opaque<T>(&self) -> *mut T {
        unsafe {
            let private_data = jsc::JSObjectGetPrivate(self.value as _);
            private_data as *mut T
        }
    }

    fn del_property(&self, key: Self) -> bool {
        let obj = self.value as jsc::JSObjectRef;
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();

        unsafe {
            let str_ref = jsc::JSValueToStringCopy(self.ctx, key.value, &mut exception);
            if !exception.is_null() {
                jsc::JSStringRelease(str_ref);
                return false;
            }

            let result = jsc::JSObjectDeleteProperty(self.ctx, obj, str_ref, &mut exception);
            jsc::JSStringRelease(str_ref);

            if !exception.is_null() {
                false
            } else {
                result
            }
        }
    }

    fn has_property(&self, key: Self) -> bool {
        let obj = self.value as jsc::JSObjectRef;
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();

        unsafe {
            let str_ref = jsc::JSValueToStringCopy(self.ctx, key.value, &mut exception);
            if !exception.is_null() {
                jsc::JSStringRelease(str_ref);
                return false;
            }

            let result = jsc::JSObjectHasProperty(self.ctx, obj, str_ref);
            jsc::JSStringRelease(str_ref);
            result
        }
    }

    fn set_property(&self, key: Self, value: Self) -> bool {
        unsafe {
            let obj = self.value as jsc::JSObjectRef;
            let mut exception: jsc::JSValueRef = std::ptr::null_mut();

            let str_ref = jsc::JSValueToStringCopy(self.ctx, key.value, &mut exception);
            if !exception.is_null() {
                jsc::JSStringRelease(str_ref);
                return false;
            }

            jsc::JSObjectSetProperty(
                self.ctx,
                obj,
                str_ref,
                value.value,
                jsc::kJSPropertyAttributeNone,
                &mut exception,
            );
            jsc::JSStringRelease(str_ref);

            exception.is_null()
        }
    }

    fn get_property(&self, key: Self) -> Option<Self> {
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();
        let obj = self.value as jsc::JSObjectRef;

        unsafe {
            let str_ref = jsc::JSValueToStringCopy(self.ctx, key.value, &mut exception);
            if !exception.is_null() {
                jsc::JSStringRelease(str_ref);
                return None;
            }

            let value = jsc::JSObjectGetProperty(self.ctx, obj, str_ref, &mut exception);
            jsc::JSStringRelease(str_ref);

            if !exception.is_null() || jsc::JSValueIsUndefined(self.ctx, value) {
                return None;
            }
            Some(JSCValue::from_owned_raw(self.ctx, value))
        }
    }

    fn define_property(
        &self,
        key: Self,
        value: Self,
        getter: Self,
        setter: Self,
        attributes: PropertyAttributes,
    ) -> bool {
        unsafe {
            let obj = self.value as jsc::JSObjectRef;
            let mut exception: jsc::JSValueRef = std::ptr::null_mut();

            let str_ref = jsc::JSValueToStringCopy(self.ctx, key.value, &mut exception);
            if !exception.is_null() {
                return false;
            }

            if attributes.has_value() {
                jsc::JSObjectSetProperty(
                    self.ctx,
                    obj,
                    str_ref,
                    value.value,
                    to_jsc_attributes(attributes),
                    &mut exception,
                );
                if exception.is_null() {
                    return false;
                }
            }
            // TODO: will handle when supporting class
            // if attributes.has_get() {}
            // if attributes.has_set() {}

            jsc::JSStringRelease(str_ref);
            true
        }
    }

    fn set_prototype(&self, prototype: Self) -> bool {
        unsafe {
            let obj = self.value as jsc::JSObjectRef;
            jsc::JSObjectSetPrototype(self.ctx, obj, prototype.value);
            true
        }
    }
}

fn to_jsc_attributes(attr: PropertyAttributes) -> u32 {
    let mut flags = jsc::kJSPropertyAttributeNone;

    if !attr.is_writable() {
        flags |= jsc::kJSPropertyAttributeReadOnly;
    }
    if !attr.is_enumerable() {
        flags |= jsc::kJSPropertyAttributeDontEnum
    }
    if !attr.is_configurable() {
        flags |= jsc::kJSPropertyAttributeDontDelete
    }

    flags
}
