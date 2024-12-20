use crate::{qjs, QJSContext, QJSValue};
use rusty_js_core::{JSClass, JSClassExt, JSContextImpl, JSValueImpl};
use std::slice;

unsafe fn prepare_args(
    ctx: *mut qjs::JSContext,
    argc: ::std::os::raw::c_int,
    argv: *mut qjs::JSValue,
) -> (QJSContext, Vec<QJSValue>) {
    // clone ctx, rust will be responsibe for managing lifetime
    let ctx = qjs::JS_DupContext(ctx);

    let args = if argc == 0 {
        Vec::new()
    } else {
        let raw_args = slice::from_raw_parts(argv, argc as usize);
        raw_args
            .iter()
            .map(|&arg| QJSValue::from_ffi(ctx, arg))
            .collect::<Vec<QJSValue>>()
    };
    (QJSContext::from_ffi(ctx), args)
}

pub(crate) unsafe extern "C" fn generic_constructor<JC>(
    ctx: *mut qjs::JSContext,
    _this: qjs::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut qjs::JSValue,
) -> qjs::JSValue
where
    JC: JSClass<QJSValue>,
{
    let (ctx, args) = prepare_args(ctx, argc, argv);
    *<JC as JSClassExt<QJSValue>>::constructor(&ctx, args.as_slice()).as_ffi_value()
}

pub(crate) unsafe extern "C" fn finalizer(_rt: *mut qjs::JSRuntime, obj: qjs::JSValue) {
    let _d = qjs::QJS_ObjectGetPrivate(obj);
}

/// FFI calling function.
pub(crate) unsafe extern "C" fn call(
    ctx: *mut qjs::JSContext,
    function: qjs::JSValue,
    _this: qjs::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut qjs::JSValue,
    _flags: ::std::os::raw::c_int,
) -> qjs::JSValue {
    let (_ctx, _args) = prepare_args(ctx, argc, argv);
    function
}
