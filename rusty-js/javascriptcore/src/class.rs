use crate::{jsc, JSCContext, JSCValue};
use rusty_js_core::{JSClass, JSClassExt, JSContextImpl, JSValueImpl};
use std::ffi::{c_char, CString};
use std::ptr;

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
        .map(move |i| JSCValue::from_borrowed_raw(raw, *arguments.add(i)))
        .collect();

    let ctx = JSCContext::from_borrowed_raw(raw);
    let value = <JC as JSClassExt<JSCValue>>::constructor(&ctx, this, args);
    if value.exception {
        if !exception.is_null() {
            *exception = value.into_raw_value();
        }
        // Return null to indicate an exception occurred
        return ptr::null_mut();
    }

    value.into_raw_value() as jsc::JSObjectRef
}

unsafe extern "C" fn finalizer<JC>(object: jsc::JSObjectRef)
where
    JC: JSClass<JSCValue>,
{
    let classid = jsc::JSObjectGetPrivate(object) as usize;
    if classid & 0x1 == 1 {
        // release JSClass
        let class_ref = classid & !0x1;
        jsc::JSClassRelease(class_ref as _);
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
            .map(|i| JSCValue::from_borrowed_raw(ctx.to_raw(), *arguments.add(i)))
            .collect()
    } else {
        vec![]
    };

    // Call the function implementation
    let value = <JC as JSClassExt<JSCValue>>::call(&ctx, function, this, args);
    if value.exception {
        if !exception.is_null() {
            *exception = value.into_raw_value();
        }
        // Return null to indicate an exception occurred
        return std::ptr::null_mut();
    }
    value.into_raw_value()
}

unsafe extern "C" fn has_instance(
    ctx: jsc::JSContextRef,
    _constructor: jsc::JSObjectRef,
    possible_instance: jsc::JSValueRef,
    _exception: *mut jsc::JSValueRef,
) -> bool {
    if jsc::JSValueIsObject(ctx, possible_instance) {
        let instance = jsc::JSValueToObject(ctx, possible_instance, std::ptr::null_mut());
        let private = jsc::JSObjectGetPrivate(instance);
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
    let class_def = jsc::JSClassDefinition {
        version: 0,
        attributes: 0,
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
        callAsConstructor: Some(generic_constructor::<JC>),
        hasInstance: Some(has_instance),
        convertToType: None,
    };

    unsafe {
        let js_class = jsc::JSClassCreate(&class_def);
        let js_class = jsc::JSClassRetain(js_class);
        let constructor = jsc::JSObjectMake(ctx.to_raw(), js_class, ptr::null_mut());

        // Very Important!
        // It's used to get JSClassRef from constructor's private data, then we can make
        // instance
        // memory is align, so we can set LSB bit to identify it's JSClass
        let classid = js_class as usize | 1;
        jsc::JSObjectSetPrivate(constructor, classid as _);

        let class_name = CString::new(class_name).unwrap();
        let class_name = jsc::JSStringCreateWithUTF8CString(class_name.as_ptr());
        let constructor_name = jsc::JSStringCreateWithUTF8CString(c"constructor".as_ptr());
        let proto_name = jsc::JSStringCreateWithUTF8CString(c"prototype".as_ptr());

        // setup constructor's attribute: name
        let nameproperty = jsc::JSStringCreateWithUTF8CString(c"name".as_ptr());
        let namevalueref = jsc::JSValueMakeString(ctx.to_raw(), class_name);
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();

        jsc::JSObjectSetProperty(
            ctx.to_raw(),
            constructor,
            nameproperty,
            namevalueref,
            jsc::kJSPropertyAttributeReadOnly | jsc::kJSPropertyAttributeDontEnum,
            &mut exception,
        );
        jsc::JSStringRelease(nameproperty);

        // Create prototype object
        let prototypeobject = jsc::JSObjectMake(ctx.to_raw(), ptr::null_mut(), ptr::null_mut());

        // Set JC::NAME.prototype
        jsc::JSObjectSetProperty(
            ctx.to_raw(),
            constructor,
            proto_name,
            prototypeobject,
            jsc::kJSPropertyAttributeDontEnum
                | jsc::kJSPropertyAttributeReadOnly
                | jsc::kJSPropertyAttributeDontDelete,
            ptr::null_mut(),
        );

        // Set JC::NAME.prototype.constructor
        jsc::JSObjectSetProperty(
            ctx.to_raw(),
            prototypeobject,
            constructor_name,
            constructor,
            jsc::kJSPropertyAttributeDontEnum,
            ptr::null_mut(),
        );

        // Get Function constructor using helper function
        let functionconstructor = get_constructor(ctx.to_raw(), c"Function".as_ptr());

        // Set JC::NAME.constructor to Function
        jsc::JSObjectSetProperty(
            ctx.to_raw(),
            constructor,
            constructor_name,
            functionconstructor,
            jsc::kJSPropertyAttributeDontEnum,
            ptr::null_mut(),
        );

        // register constructor function to global object
        let global = jsc::JSContextGetGlobalObject(ctx.to_raw());
        jsc::JSObjectSetProperty(
            ctx.to_raw(),
            global,
            class_name,
            constructor,
            jsc::kJSPropertyAttributeNone,
            ptr::null_mut(),
        );
        jsc::JSStringRelease(class_name);
        jsc::JSStringRelease(constructor_name);
        jsc::JSStringRelease(proto_name);

        JSCValue::from_owned_obj(ctx.to_raw(), constructor)
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
