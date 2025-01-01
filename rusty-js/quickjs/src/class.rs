use crate::{qjs, QJSContext, QJSValue};
use rusty_js_core::{JSClass, JSClassExt, JSContextImpl, JSValueImpl};

pub(crate) unsafe extern "C" fn generic_constructor<JC>(
    ctx: *mut qjs::JSContext,
    this: qjs::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut qjs::JSValue,
) -> qjs::JSValue
where
    JC: JSClass<QJSValue>,
{
    qjs::JS_DupValue(ctx, this);
    let this = QJSValue::from_ffi(ctx, this);

    let args: Vec<_> = (0..argc as usize)
        .map(move |i| {
            qjs::JS_DupValue(ctx, *argv.add(i));
            QJSValue::from_ffi(ctx, *argv.add(i))
        })
        .collect();

    let ctx = QJSContext::from_ffi(ctx);
    <JC as JSClassExt<QJSValue>>::constructor(&ctx, this, args).into_ffi_value()
}

pub(crate) unsafe extern "C" fn finalizer<JC>(_rt: *mut qjs::JSRuntime, obj: qjs::JSValue)
where
    JC: JSClass<QJSValue>,
{
    let ctx: *mut qjs::JSContext = std::ptr::null_mut();
    qjs::JS_DupValue(ctx, obj);

    let value = QJSValue::from_ffi(ctx, obj);
    <JC as JSClassExt<QJSValue>>::free(value);
}

/// FFI calling function.
pub(crate) unsafe extern "C" fn call<JC>(
    ctx: *mut qjs::JSContext,
    function: qjs::JSValue,
    this: qjs::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut qjs::JSValue,
    _flags: ::std::os::raw::c_int,
) -> qjs::JSValue
where
    JC: JSClass<QJSValue>,
{
    // in FFI context, JS engine has ownship for function, argv etc. Since QJSValue has drop,
    // we have to increase reference count firstly to make rust has its ownship.
    qjs::JS_DupValue(ctx, function);
    qjs::JS_DupValue(ctx, this);

    let this = QJSValue::from_ffi(ctx, this);
    let function = QJSValue::from_ffi(ctx, function);
    let args: Vec<_> = (0..argc as usize)
        .map(move |i| {
            qjs::JS_DupValue(ctx, *argv.add(i));
            QJSValue::from_ffi(ctx, *argv.add(i))
        })
        .collect();

    let ctx = QJSContext::from_ffi(ctx);
    <JC as JSClassExt<QJSValue>>::call(&ctx, function, this, args).into_ffi_value()
}
