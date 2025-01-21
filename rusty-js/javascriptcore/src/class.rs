use crate::{jsc, JSCContext, JSCValue};
use rusty_js_core::{JSClass, JSClassExt, JSContextImpl, JSValueImpl};

pub(crate) unsafe extern "C" fn generic_constructor<JC>(
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
        return std::ptr::null_mut();
    }

    value.into_raw_value() as jsc::JSObjectRef
}

pub(crate) unsafe extern "C" fn finalizer<JC>(object: jsc::JSObjectRef)
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

pub(crate) unsafe extern "C" fn call_as_function<JC>(
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
