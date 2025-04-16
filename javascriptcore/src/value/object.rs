use crate::{jsc, JSCValue};
use rong_core::{JSObjectOps, JSValueImpl, PropertyAttributes};

impl JSObjectOps for JSCValue {
    fn new_object(ctx: &Self::Context) -> Self {
        unsafe {
            let obj = jsc::JSObjectMake(ctx.to_raw(), std::ptr::null_mut(), std::ptr::null_mut());
            JSCValue::from_owned_obj(ctx.to_raw(), obj)
        }
    }

    fn make_instance(ctx: &Self::Context, constructor: Self, data: *mut ()) -> Self {
        let obj = unsafe {
            let classref = crate::class::get_classref_by_constructor(constructor);
            jsc::JSObjectMake(ctx.to_raw(), classref as jsc::JSClassRef, data as _)
        };

        JSCValue::from_owned_obj(ctx.to_raw(), obj)
    }

    fn get_opaque(&self) -> *mut () {
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

    fn set_prototype(&self, prototype: Self) -> bool {
        unsafe {
            let obj = self.as_obj();
            jsc::JSObjectSetPrototype(self.ctx, obj, prototype.as_value());
            true
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
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();

        // Get the Object constructor
        let object_ctor = crate::class::get_constructor(self.ctx, c"Object".as_ptr());

        unsafe {
            // Create property descriptor
            let descriptor =
                jsc::JSObjectMake(self.ctx, std::ptr::null_mut(), std::ptr::null_mut());

            if attributes.has_value() {
                let value_str = jsc::JSStringCreateWithUTF8CString(c"value".as_ptr());
                jsc::JSObjectSetProperty(
                    self.ctx,
                    descriptor,
                    value_str,
                    value.as_value(),
                    jsc::kJSPropertyAttributeNone,
                    &mut exception,
                );
                jsc::JSStringRelease(value_str);
                if !exception.is_null() {
                    return false;
                }
            }

            if attributes.has_get() {
                let get_str = jsc::JSStringCreateWithUTF8CString(c"get".as_ptr());
                jsc::JSObjectSetProperty(
                    self.ctx,
                    descriptor,
                    get_str,
                    getter.as_value(),
                    jsc::kJSPropertyAttributeNone,
                    &mut exception,
                );
                jsc::JSStringRelease(get_str);
                if !exception.is_null() {
                    return false;
                }
            }

            if attributes.has_set() {
                let set_str = jsc::JSStringCreateWithUTF8CString(c"set".as_ptr());
                jsc::JSObjectSetProperty(
                    self.ctx,
                    descriptor,
                    set_str,
                    setter.as_value(),
                    jsc::kJSPropertyAttributeNone,
                    &mut exception,
                );
                jsc::JSStringRelease(set_str);
                if !exception.is_null() {
                    return false;
                }
            }

            // Add enumerable/configurable/writable flags
            let flags = to_jsc_attributes(attributes);
            let enumerable_str = jsc::JSStringCreateWithUTF8CString(c"enumerable".as_ptr());
            jsc::JSObjectSetProperty(
                self.ctx,
                descriptor,
                enumerable_str,
                jsc::JSValueMakeBoolean(self.ctx, (flags & jsc::kJSPropertyAttributeDontEnum) == 0),
                jsc::kJSPropertyAttributeNone,
                &mut exception,
            );
            jsc::JSStringRelease(enumerable_str);

            let configurable_str = jsc::JSStringCreateWithUTF8CString(c"configurable".as_ptr());
            jsc::JSObjectSetProperty(
                self.ctx,
                descriptor,
                configurable_str,
                jsc::JSValueMakeBoolean(
                    self.ctx,
                    (flags & jsc::kJSPropertyAttributeDontDelete) == 0,
                ),
                jsc::kJSPropertyAttributeNone,
                &mut exception,
            );
            jsc::JSStringRelease(configurable_str);

            if attributes.has_value() {
                let writable_str = jsc::JSStringCreateWithUTF8CString(c"writable".as_ptr());
                jsc::JSObjectSetProperty(
                    self.ctx,
                    descriptor,
                    writable_str,
                    jsc::JSValueMakeBoolean(
                        self.ctx,
                        (flags & jsc::kJSPropertyAttributeReadOnly) == 0,
                    ),
                    jsc::kJSPropertyAttributeNone,
                    &mut exception,
                );
                jsc::JSStringRelease(writable_str);
            }

            // Get defineProperty function
            let define_prop_str = jsc::JSStringCreateWithUTF8CString(c"defineProperty".as_ptr());
            let define_prop_fn =
                jsc::JSObjectGetProperty(self.ctx, object_ctor, define_prop_str, &mut exception);
            jsc::JSStringRelease(define_prop_str);

            if !exception.is_null() || jsc::JSValueIsUndefined(self.ctx, define_prop_fn) {
                return false;
            }

            // Call defineProperty
            let args = [self.as_value(), key.as_value(), descriptor];

            jsc::JSObjectCallAsFunction(
                self.ctx,
                define_prop_fn as jsc::JSObjectRef,
                std::ptr::null_mut(),
                args.len(),
                args.as_ptr(),
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

    fn instance_of(&self, constructor: Self) -> bool {
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();
        unsafe {
            let result = jsc::JSValueIsInstanceOfConstructor(
                self.ctx,
                self.as_value(),
                constructor.as_obj(),
                &mut exception,
            );
            exception.is_null() && result
        }
    }

    fn get_own_property_names(&self) -> Option<Vec<Self>> {
        unsafe {
            let mut exception: jsc::JSValueRef = std::ptr::null_mut();

            // Get the property names array
            let names = jsc::JSObjectCopyPropertyNames(self.ctx, self.as_obj());
            if names.is_null() {
                return None;
            }

            let count = jsc::JSPropertyNameArrayGetCount(names);
            let mut properties = Vec::with_capacity(count as usize);

            // Get the Object constructor
            let object_ctor = crate::class::get_constructor(self.ctx, c"Object".as_ptr());

            // Get the prototype of the Object constructor
            let prototype = jsc::JSObjectGetPrototype(self.ctx, object_ctor);
            let prototype = jsc::JSValueToObject(self.ctx, prototype, &mut exception);

            // Get the hasOwnProperty function
            let has_own_str = jsc::JSStringCreateWithUTF8CString(c"hasOwnProperty".as_ptr());
            let has_own =
                jsc::JSObjectGetProperty(self.ctx, prototype as _, has_own_str, &mut exception);
            jsc::JSStringRelease(has_own_str);

            if !exception.is_null() {
                return None;
            }

            // Collect all property names
            for i in 0..count {
                let name = jsc::JSPropertyNameArrayGetNameAtIndex(names, i);
                let value = jsc::JSValueMakeString(self.ctx, name);

                // Call hasOwnProperty on the current object
                let args = [value];
                let result = jsc::JSObjectCallAsFunction(
                    self.ctx,
                    has_own as jsc::JSObjectRef,
                    self.as_obj(),
                    args.len(),
                    args.as_ptr(),
                    &mut exception,
                );

                if exception.is_null() {
                    let is_own = jsc::JSValueToBoolean(self.ctx, result);

                    if is_own {
                        properties.push(JSCValue::from_owned_raw(self.ctx, value));
                    }
                }
            }

            jsc::JSPropertyNameArrayRelease(names);
            Some(properties)
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
