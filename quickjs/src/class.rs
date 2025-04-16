use crate::{qjs, QJSContext, QJSValue};
use rong_core::{JSClass, JSClassExt, JSContextImpl, JSTypeOf, JSValueImpl};

pub(crate) unsafe extern "C" fn generic_constructor<JC>(
    ctx: *mut qjs::JSContext,
    this: qjs::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut qjs::JSValue,
) -> qjs::JSValue
where
    JC: JSClass<QJSValue>,
{
    let this = QJSValue::from_borrowed_raw(ctx, this);

    let args: Vec<_> = (0..argc as usize)
        .map(move |i| QJSValue::from_borrowed_raw(ctx, *argv.add(i)))
        .collect();

    let ctx = QJSContext::from_borrowed_raw(ctx);
    let value = <JC as JSClassExt<QJSValue>>::constructor(&ctx, this, args);
    if value.is_exception() {
        qjs::JS_Throw(ctx.to_raw(), value.into_raw_value())
    } else {
        value.into_raw_value()
    }
}

pub(crate) unsafe extern "C" fn finalizer<JC>(_rt: *mut qjs::JSRuntime, obj: qjs::JSValue)
where
    JC: JSClass<QJSValue>,
{
    let ctx: *mut qjs::JSContext = std::ptr::null_mut();
    let value = QJSValue::from_borrowed_raw(ctx, obj);
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
    let this = QJSValue::from_borrowed_raw(ctx, this);
    let function = QJSValue::from_borrowed_raw(ctx, function);
    let args: Vec<_> = (0..argc as usize)
        .map(move |i| QJSValue::from_borrowed_raw(ctx, *argv.add(i)))
        .collect();

    let ctx = QJSContext::from_borrowed_raw(ctx);
    let value = <JC as JSClassExt<QJSValue>>::call(&ctx, function, this, args);
    if value.is_exception() {
        qjs::JS_Throw(ctx.to_raw(), value.into_raw_value())
    } else {
        value.into_raw_value()
    }
}
