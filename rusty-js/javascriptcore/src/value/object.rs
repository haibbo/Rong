use crate::{jsc, JSCValue};
use rusty_js_core::{JSObjectOps, JSValueImpl, PropertyAttributes};

fn make_instance(
    ctx: *mut jsc::OpaqueJSContext,
    constructor: JSCValue,
    data: *mut (),
) -> jsc::JSObjectRef {
    unsafe {
        // must clear LSB bit
        let classref = jsc::JSObjectGetPrivate(constructor.as_obj()) as usize & !0x1;
        jsc::JSObjectMake(ctx, classref as jsc::JSClassRef, data as _)
    }
}

impl JSObjectOps for JSCValue {
    fn new_object(ctx: &Self::Context) -> Self {
        unsafe {
            let obj = jsc::JSObjectMake(ctx.to_raw(), std::ptr::null_mut(), std::ptr::null_mut());
            JSCValue::from_owned_obj(ctx.to_raw(), obj)
        }
    }

    fn make_instance(ctx: &Self::Context, constructor: Self, data: *mut ()) -> Self {
        // println!("set: {} {:?}", std::any::type_name::<T>(), data);
        let obj = make_instance(ctx.to_raw(), constructor, data);
        JSCValue::from_owned_obj(ctx.to_raw(), obj)
    }

    fn get_opaque(&self) -> *mut () {
        // println!("get: {} {:?}", std::any::type_name::<T>(), private_data);
        unsafe {
            let private_data = jsc::JSObjectGetPrivate(self.as_obj());
            private_data as _
        }
    }

    fn del_property(&self, key: Self) -> bool {
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();

        unsafe {
            let result = jsc::JSObjectDeletePropertyForKey(
                self.ctx,
                self.as_obj(),
                key.as_value(),
                &mut exception,
            );
            exception.is_null() && result
        }
    }

    fn has_property(&self, key: Self) -> bool {
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();

        unsafe {
            let result = jsc::JSObjectHasPropertyForKey(
                self.ctx,
                self.as_obj(),
                key.as_value(),
                &mut exception,
            );
            exception.is_null() && result
        }
    }

    fn set_property(&self, key: Self, value: Self) -> bool {
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();
        unsafe {
            jsc::JSObjectSetPropertyForKey(
                self.ctx,
                self.as_obj(),
                key.as_value(),
                value.as_value(),
                jsc::kJSPropertyAttributeNone,
                &mut exception,
            );

            exception.is_null()
        }
    }

    fn get_property(&self, key: Self) -> Option<Self> {
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();

        unsafe {
            let value = jsc::JSObjectGetPropertyForKey(
                self.ctx,
                self.as_obj(),
                key.as_value(),
                &mut exception,
            );
            if !exception.is_null() || jsc::JSValueIsUndefined(self.ctx, value) {
                None
            } else {
                Some(JSCValue::from_owned_raw(self.ctx, value))
            }
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
            let mut exception: jsc::JSValueRef = std::ptr::null_mut();

            let str_ref = jsc::JSValueToStringCopy(self.ctx, key.as_value(), &mut exception);
            if !exception.is_null() {
                return false;
            }

            let obj = self.as_obj();
            if attributes.has_value() {
                jsc::JSObjectSetProperty(
                    self.ctx,
                    obj,
                    str_ref,
                    value.as_value(),
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
            let obj = self.as_obj();
            jsc::JSObjectSetPrototype(self.ctx, obj, prototype.as_value());
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
