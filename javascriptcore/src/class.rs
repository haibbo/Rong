use crate::{JSCContext, JSCValue, jsc};
use rong_core::{JSClass, JSClassExt, JSContextImpl, JSTypeOf, JSValueImpl};
use std::collections::HashMap;
use std::ffi::{CString, c_char};
use std::ptr;
use std::sync::{LazyLock, RwLock};

/// Global storage mapping constructor objects to their corresponding class references
///
/// This maintains a mapping between:
/// - Key: Constructor object pointer (as usize)
/// - Value: Class reference pointer (as usize)
///
/// The mapping is thread-safe through RwLock and initialized lazily
static CLASS: LazyLock<RwLock<HashMap<usize, usize>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Retrieves the class reference associated with a given constructor
//
/// # Returns
/// The corresponding class reference if found, otherwise null pointer
pub(crate) fn get_classref_by_constructor(constructor: JSCValue) -> jsc::JSClassRef {
    let constructor_ptr = constructor.as_value() as usize;
    if let Ok(map) = CLASS.read() {
        if let Some(&class_ref) = map.get(&constructor_ptr) {
            return class_ref as jsc::JSClassRef;
        }
    }
    ptr::null_mut()
}

unsafe extern "C" fn generic_constructor<JC>(
    ctx: jsc::JSContextRef,
    constructor: jsc::JSObjectRef,
    argument_count: usize,
    arguments: *const jsc::JSValueRef,
    exception: *mut jsc::JSValueRef,
) -> jsc::JSObjectRef
where
    JC: JSClass<JSCValue>,
{
    let raw: *mut jsc::OpaqueJSContext = ctx as _;
    let this = JSCValue::from_borrowed_obj(raw, constructor);

    let args: Vec<_> = (0..argument_count)
        .map(move |i| unsafe { JSCValue::from_borrowed_raw(raw, *arguments.add(i)) })
        .collect();

    let ctx = JSCContext::from_borrowed_raw(raw);
    let value = <JC as JSClassExt<JSCValue>>::constructor(&ctx, this, args);
    if value.is_exception() {
        if !exception.is_null() {
            unsafe {
                *exception = value.into_raw_value();
            }
        }
        unsafe {
            return jsc::JSValueMakeUndefined(ctx.to_raw()) as _;
        }
    }

    value.into_raw_value() as jsc::JSObjectRef
}

unsafe extern "C" fn finalizer<JC>(object: jsc::JSObjectRef)
where
    JC: JSClass<JSCValue>,
{
    let classid = unsafe { jsc::JSObjectGetPrivate(object) } as usize;
    if classid & 0x1 == 1 {
        // release JSClass
        let class_ref = classid & !0x1;
        unsafe {
            jsc::JSClassRelease(class_ref as _);
        }
        return;
    }

    let ctx: *mut jsc::OpaqueJSContext = std::ptr::null_mut();
    let value = JSCValue::from_borrowed_obj(ctx, object);
    <JC as JSClassExt<JSCValue>>::free(value);
}

unsafe extern "C" fn call_as_function<JC>(
    ctx: jsc::JSContextRef,
    function: jsc::JSObjectRef,
    this_object: jsc::JSObjectRef,
    argument_count: usize,
    arguments: *const jsc::JSValueRef,
    exception: *mut jsc::JSValueRef,
) -> jsc::JSValueRef
where
    JC: JSClass<JSCValue>,
{
    let ctx = JSCContext::from_borrowed_raw(ctx as _);
    let function = JSCValue::from_borrowed_obj(ctx.to_raw(), function);
    let this = JSCValue::from_borrowed_obj(ctx.to_raw(), this_object);

    // Convert arguments to Vec<JSCValue>
    let args: Vec<JSCValue> = if !arguments.is_null() {
        (0..argument_count)
            .map(|i| unsafe { JSCValue::from_borrowed_raw(ctx.to_raw(), *arguments.add(i)) })
            .collect()
    } else {
        vec![]
    };

    // Call the function implementation
    let value = <JC as JSClassExt<JSCValue>>::call(&ctx, function, this, args);
    if value.is_exception() {
        if !exception.is_null() {
            unsafe {
                *exception = value.into_raw_value();
            }
        }
        unsafe {
            return jsc::JSValueMakeUndefined(ctx.to_raw());
        }
    }
    value.into_raw_value()
}

unsafe extern "C" fn has_instance(
    ctx: jsc::JSContextRef,
    _constructor: jsc::JSObjectRef,
    possible_instance: jsc::JSValueRef,
    _exception: *mut jsc::JSValueRef,
) -> bool {
    if unsafe { jsc::JSValueIsObject(ctx, possible_instance) } {
        let instance =
            unsafe { jsc::JSValueToObject(ctx, possible_instance, std::ptr::null_mut()) };
        let private = unsafe { jsc::JSObjectGetPrivate(instance) };
        if !private.is_null() {
            return true;
        }
    }

    false
}

/// Registers a JavaScript class with the given context and class name.
///
/// This function creates a JavaScript class using the provided class name,
/// sets up the constructor and prototype, and registers it in the global object.
///
/// # Arguments
///
/// * `ctx` - The JavaScript context.
/// * `class_name` - The name of the class.
///
/// # Returns
///
/// The constructor function for the registered class.
pub(crate) fn register_class_internal<JC>(ctx: &JSCContext, class_name: &str) -> JSCValue
where
    JC: JSClass<JSCValue>,
{
    let class_name_cstr = CString::new(class_name).unwrap();
    let ctx = ctx.to_raw();

    unsafe {
        // It is not possible to use JS subclassing with objects created from
        // a class definition that sets callAsConstructor by default.
        // Subclassing is supported via the JSObjectMakeConstructor function, however.
        let class_def = jsc::JSClassDefinition {
            version: 0,
            attributes: jsc::kJSClassAttributeNone,
            className: class_name_cstr.as_ptr(),
            parentClass: ptr::null_mut(),
            staticValues: ptr::null(),
            staticFunctions: ptr::null(),
            initialize: None,
            finalize: Some(finalizer::<JC>),
            hasProperty: None,
            getProperty: None,
            setProperty: None,
            deleteProperty: None,
            getPropertyNames: None,
            callAsFunction: Some(call_as_function::<JC>),
            callAsConstructor: None,
            hasInstance: Some(has_instance),
            convertToType: None,
        };

        let js_export = jsc::JSClassCreate(&class_def);
        let constructor =
            jsc::JSObjectMakeConstructor(ctx, js_export, Some(generic_constructor::<JC>));

        // set constructor'name to class name
        let name_key = jsc::JSStringCreateWithUTF8CString(c"name".as_ptr());
        let class_name = jsc::JSStringCreateWithUTF8CString(class_name_cstr.as_ptr());
        let name_value = jsc::JSValueMakeString(ctx, class_name);
        jsc::JSObjectSetProperty(
            ctx,
            constructor,
            name_key,
            name_value,
            jsc::kJSPropertyAttributeReadOnly | jsc::kJSPropertyAttributeDontEnum,
            ptr::null_mut(),
        );
        jsc::JSStringRelease(name_key);
        jsc::JSStringRelease(class_name);

        // set JC.constructor to Function
        let function = get_constructor(ctx, c"Function".as_ptr());
        let constructor_key = jsc::JSStringCreateWithUTF8CString(c"constructor".as_ptr());
        jsc::JSObjectSetProperty(
            ctx,
            constructor,
            constructor_key,
            function,
            jsc::kJSPropertyAttributeDontEnum,
            ptr::null_mut(),
        );

        // set constructor's prototype.constructor to constructor
        let prototype_key = jsc::JSStringCreateWithUTF8CString(c"prototype".as_ptr());
        let prototype = jsc::JSObjectGetProperty(ctx, constructor, prototype_key, ptr::null_mut());
        let prototype = jsc::JSValueToObject(ctx, prototype, ptr::null_mut());
        jsc::JSObjectSetProperty(
            ctx,
            prototype,
            constructor_key,
            constructor,
            jsc::kJSPropertyAttributeDontEnum,
            ptr::null_mut(),
        );

        jsc::JSStringRelease(prototype_key);
        jsc::JSStringRelease(constructor_key);

        // constructor built by JSObjectMakeConstructor does not support JSObjectSetProperty, we
        // have to setup map ourelf
        CLASS
            .write()
            .unwrap()
            .insert(constructor as usize, js_export as usize);

        JSCValue::from_owned_obj(ctx, constructor)
    }
}

pub(crate) fn get_constructor(
    ctx: *mut jsc::OpaqueJSContext,
    name: *const c_char,
) -> jsc::JSObjectRef {
    unsafe {
        let global = jsc::JSContextGetGlobalObject(ctx);
        let js_name = jsc::JSStringCreateWithUTF8CString(name);
        let value = jsc::JSObjectGetProperty(ctx, global, js_name, ptr::null_mut());
        jsc::JSStringRelease(js_name);
        jsc::JSValueToObject(ctx, value, ptr::null_mut())
    }
}
