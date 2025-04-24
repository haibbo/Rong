use crate::{QJSContext, QJSValue, qjs};
use rong_core::{JSClass, JSClassExt, JSContextImpl, JSTypeOf, JSValueImpl};
use std::cell::RefCell;

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
        .map(move |i| unsafe { QJSValue::from_borrowed_raw(ctx, *argv.add(i)) })
        .collect();

    let ctx = QJSContext::from_borrowed_raw(ctx);
    let value = <JC as JSClassExt<QJSValue>>::constructor(&ctx, this, args);
    if value.is_exception() {
        unsafe { qjs::JS_Throw(ctx.to_raw(), value.into_raw_value()) }
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

/// GC mark function for QuickJS
/// This function is called by the QuickJS GC when marking objects
/// It collects and marks JavaScript values held by Rust objects
pub(crate) unsafe extern "C" fn gc_mark<JC>(
    rt: *mut qjs::JSRuntime,
    val: qjs::JSValue,
    mark_func: qjs::JS_MarkFunc,
) where
    JC: JSClass<QJSValue>,
{
    unsafe {
        // Extract the Rust object and call collect_js_references on it
        let ptr = qjs::QJS_ObjectGetPrivate(val) as *mut RefCell<JC>;
        if !ptr.is_null() {
            if let Ok(borrowed) = (*ptr).try_borrow() {
                // Mark each value
                for root in borrowed.gc_mark() {
                    let v = *root.as_value().as_raw_value();
                    qjs::JS_MarkValue(rt, v, mark_func);
                }
            }
        }
    }
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
        .map(move |i| unsafe { QJSValue::from_borrowed_raw(ctx, *argv.add(i)) })
        .collect();

    let ctx = QJSContext::from_borrowed_raw(ctx);
    let value = <JC as JSClassExt<QJSValue>>::call(&ctx, function, this, args);
    if value.is_exception() {
        unsafe { qjs::JS_Throw(ctx.to_raw(), value.into_raw_value()) }
    } else {
        value.into_raw_value()
    }
}
