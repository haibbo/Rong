use crate::{JSCRuntime, JSCValue, jsc};
use rong_core::{
    JSClass, JSContextImpl, JSErrorFactory, JSExceptionThrower, JSRuntimeImpl, JSValueImpl,
    RongJSError,
};
use std::ffi::CString;
use std::ptr;

pub struct JSCContext {
    raw: *mut jsc::OpaqueJSContext,
}

impl JSContextImpl for JSCContext {
    type RawContext = *mut jsc::OpaqueJSContext;
    type Runtime = JSCRuntime;
    type Value = JSCValue;

    fn new(runtime: &Self::Runtime) -> Self {
        Self {
            raw: unsafe { jsc::JSGlobalContextCreateInGroup(runtime.to_raw(), ptr::null_mut()) },
        }
    }

    fn as_raw(&self) -> &Self::RawContext {
        &self.raw
    }

    fn context_id(ctx: &Self::RawContext) -> usize {
        *ctx as *const _ as usize
    }

    fn from_borrowed_raw(ctx: Self::RawContext) -> Self {
        Self::_from_borrowed_raw(ctx)
    }

    fn eval(&self, source: rong_core::Source) -> Self::Value {
        let filename = source.name().unwrap_or("eval");
        // Keep engine behavior consistent with QuickJS backend, which evaluates in strict mode
        // (it sets JS_EVAL_FLAG_STRICT by default).
        //
        // JavaScriptCore's JSEvaluateScript does not provide a strict-mode flag, so we enforce it
        // by prepending a directive prologue.
        let mut code_bytes = b"\"use strict\";\n".to_vec();
        code_bytes.extend_from_slice(source.code());
        let code = CString::new(code_bytes).unwrap();
        let c_filename = CString::new(filename).unwrap();

        unsafe {
            let js_str = jsc::JSStringCreateWithUTF8CString(code.as_ptr());
            let js_filename = jsc::JSStringCreateWithUTF8CString(c_filename.as_ptr());

            let mut exception: jsc::JSValueRef = ptr::null_mut();
            let result = jsc::JSEvaluateScript(
                self.raw,
                js_str,
                std::ptr::null_mut(), // thisObject (null means use global object)
                js_filename,
                1,
                &mut exception,
            );

            jsc::JSStringRelease(js_str);
            jsc::JSStringRelease(js_filename);

            // Check if an exception occurred
            if !exception.is_null() {
                // println!("got exception");
                return JSCValue::from_owned_raw(self.raw, exception).with_exception();
            }
            // println!(
            //     "Result isObject: {}",
            //     jsc::JSValueIsObject(self.raw, result)
            // );
            JSCValue::from_owned_raw(self.raw, result)
        }
    }

    fn global(&self) -> Self::Value {
        unsafe {
            let global_obj = jsc::JSContextGetGlobalObject(self.raw);
            JSCValue::from_owned_obj(self.raw, global_obj)
        }
    }

    /// Registers a JavaScript class in the context.
    fn register_class<JC>(&self) -> Self::Value
    where
        JC: JSClass<Self::Value>,
    {
        crate::class::register_class_internal::<JC>(self, JC::NAME)
    }

    /// Calls a JavaScript function with the specified `this` value and arguments.
    fn call(
        &self,
        function: &Self::Value,
        this: Self::Value,
        argv: Vec<Self::Value>,
    ) -> Self::Value {
        unsafe {
            let mut exception: jsc::JSValueRef = std::ptr::null_mut();

            let function_obj = function.as_obj();
            let this_value: jsc::JSValueRef = (*this.as_raw_value()).cast();

            // Convert argv to raw JSValues
            let args: Vec<jsc::JSValueRef> = argv
                .iter()
                .map(|v| {
                    let raw = *v.as_raw_value();
                    raw.cast()
                })
                .collect();

            // Fast path: object `this` can be passed directly.
            // For non-object `this` (undefined/null/primitive), we must use
            // Function.prototype.call so JavaScriptCore applies correct this-binding rules.
            let result = if jsc::JSValueIsObject(self.raw, this_value) {
                let this_obj = jsc::JSValueToObject(self.raw, this_value, &mut exception);
                if !exception.is_null() {
                    return JSCValue::from_owned_raw(self.raw, exception).with_exception();
                }
                jsc::JSObjectCallAsFunction(
                    self.raw,
                    function_obj,
                    this_obj,
                    args.len(),
                    args.as_ptr(),
                    &mut exception,
                )
            } else {
                // Use Function.prototype.call instead of function.call to avoid user overrides.
                let global_obj = jsc::JSContextGetGlobalObject(self.raw);

                let function_key = jsc::JSStringCreateWithUTF8CString(c"Function".as_ptr());
                let function_value =
                    jsc::JSObjectGetProperty(self.raw, global_obj, function_key, &mut exception);
                jsc::JSStringRelease(function_key);
                if !exception.is_null() {
                    return JSCValue::from_owned_raw(self.raw, exception).with_exception();
                }

                let function_ctor = jsc::JSValueToObject(self.raw, function_value, &mut exception);
                if !exception.is_null() {
                    return JSCValue::from_owned_raw(self.raw, exception).with_exception();
                }

                let prototype_key = jsc::JSStringCreateWithUTF8CString(c"prototype".as_ptr());
                let prototype_value = jsc::JSObjectGetProperty(
                    self.raw,
                    function_ctor,
                    prototype_key,
                    &mut exception,
                );
                jsc::JSStringRelease(prototype_key);
                if !exception.is_null() {
                    return JSCValue::from_owned_raw(self.raw, exception).with_exception();
                }

                let prototype_obj = jsc::JSValueToObject(self.raw, prototype_value, &mut exception);
                if !exception.is_null() {
                    return JSCValue::from_owned_raw(self.raw, exception).with_exception();
                }

                let call_key = jsc::JSStringCreateWithUTF8CString(c"call".as_ptr());
                let call_value =
                    jsc::JSObjectGetProperty(self.raw, prototype_obj, call_key, &mut exception);
                jsc::JSStringRelease(call_key);
                if !exception.is_null() {
                    return JSCValue::from_owned_raw(self.raw, exception).with_exception();
                }

                let call_obj = jsc::JSValueToObject(self.raw, call_value, &mut exception);
                if !exception.is_null() {
                    return JSCValue::from_owned_raw(self.raw, exception).with_exception();
                }

                let mut call_args: Vec<jsc::JSValueRef> = Vec::with_capacity(args.len() + 1);
                call_args.push(this_value);
                call_args.extend(args);

                jsc::JSObjectCallAsFunction(
                    self.raw,
                    call_obj,
                    function_obj,
                    call_args.len(),
                    call_args.as_ptr(),
                    &mut exception,
                )
            };

            if !exception.is_null() {
                return JSCValue::from_owned_raw(self.raw, exception).with_exception();
            }

            JSCValue::from_owned_raw(self.raw, result)
        }
    }

    fn promise(&self) -> (Self::Value, Self::Value, Self::Value) {
        unsafe {
            let mut resolve_fn: jsc::JSObjectRef = std::ptr::null_mut();
            let mut reject_fn: jsc::JSObjectRef = std::ptr::null_mut();
            let mut exception: jsc::JSValueRef = std::ptr::null_mut();

            let promise = jsc::JSObjectMakeDeferredPromise(
                self.raw,
                &mut resolve_fn,
                &mut reject_fn,
                &mut exception,
            );

            if !exception.is_null() {
                let undefined = jsc::JSValueMakeUndefined(self.raw);
                return (
                    JSCValue::from_owned_raw(self.raw, undefined),
                    JSCValue::from_owned_raw(self.raw, undefined),
                    JSCValue::from_owned_raw(self.raw, undefined),
                );
            }

            (
                JSCValue::from_borrowed_obj(self.raw, promise),
                JSCValue::from_borrowed_obj(self.raw, resolve_fn),
                JSCValue::from_borrowed_obj(self.raw, reject_fn),
            )
        }
    }

    fn compile_to_bytecode(&self, _source: rong_core::Source) -> Result<Vec<u8>, RongJSError> {
        Err(RongJSError::NotSupportByteCode())
    }

    fn run_bytecode(&self, _bytes: &[u8]) -> Self::Value {
        todo!()
    }
}

impl JSCContext {
    fn _from_borrowed_raw(ctx: *mut jsc::OpaqueJSContext) -> Self {
        let raw = unsafe { jsc::JSGlobalContextRetain(ctx) };
        Self { raw }
    }

    pub(crate) fn to_raw(&self) -> *mut jsc::OpaqueJSContext {
        self.raw
    }
}

impl Drop for JSCContext {
    fn drop(&mut self) {
        unsafe {
            jsc::JSGlobalContextRelease(self.raw);
        }
    }
}

impl Clone for JSCContext {
    fn clone(&self) -> Self {
        unsafe {
            // Retains a global JavaScript execution context.
            jsc::JSGlobalContextRetain(self.raw);
            Self { raw: self.raw }
        }
    }
}

impl JSErrorFactory for JSCContext {
    fn new_error(&self, name: &str, message: impl AsRef<str>, code: Option<&str>) -> Self::Value {
        self.new_error_with_name_internal(name, message.as_ref(), code)
    }
}

impl JSExceptionThrower for JSCContext {
    fn throw(&self, value: Self::Value) -> Self::Value {
        value.with_exception()
    }
}

impl JSCContext {
    pub(crate) fn new_error_with_name_internal(
        &self,
        error_name: &str,
        message: &str,
        code: Option<&str>,
    ) -> JSCValue {
        let message_cstr = CString::new(message).unwrap();
        let error_name_cstr = CString::new(error_name).unwrap();

        unsafe {
            let message_str = jsc::JSStringCreateWithUTF8CString(message_cstr.as_ptr());
            let error_name_str = jsc::JSStringCreateWithUTF8CString(error_name_cstr.as_ptr());

            let mut exception: jsc::JSValueRef = ptr::null_mut();
            let global = jsc::JSContextGetGlobalObject(self.raw);
            let ctor_value =
                jsc::JSObjectGetProperty(self.raw, global, error_name_str, &mut exception);

            let mut error: jsc::JSObjectRef = ptr::null_mut();
            if exception.is_null()
                && !ctor_value.is_null()
                && jsc::JSValueIsObject(self.raw, ctor_value)
            {
                let ctor = jsc::JSValueToObject(self.raw, ctor_value, &mut exception);
                if exception.is_null() && !ctor.is_null() {
                    let message_value = jsc::JSValueMakeString(self.raw, message_str);
                    let args = [message_value];
                    let obj = jsc::JSObjectCallAsConstructor(
                        self.raw,
                        ctor,
                        1,
                        args.as_ptr(),
                        &mut exception,
                    );
                    if exception.is_null() && !obj.is_null() {
                        let proto_key = jsc::JSStringCreateWithUTF8CString(c"prototype".as_ptr());
                        let proto_value =
                            jsc::JSObjectGetProperty(self.raw, ctor, proto_key, &mut exception);
                        if exception.is_null() && !proto_value.is_null() {
                            jsc::JSObjectSetPrototype(self.raw, obj, proto_value);
                        }
                        jsc::JSStringRelease(proto_key);

                        let name_key = jsc::JSStringCreateWithUTF8CString(c"name".as_ptr());
                        let name_value = jsc::JSValueMakeString(self.raw, error_name_str);
                        jsc::JSObjectSetProperty(
                            self.raw,
                            obj,
                            name_key,
                            name_value,
                            jsc::kJSPropertyAttributeDontEnum,
                            ptr::null_mut(),
                        );
                        jsc::JSStringRelease(name_key);

                        error = obj;
                    }
                }
            }

            if error.is_null() {
                let message_value = jsc::JSValueMakeString(self.raw, message_str);
                let args = [message_value];
                let obj = jsc::JSObjectMakeError(self.raw, 1, args.as_ptr(), ptr::null_mut());

                let name_key = jsc::JSStringCreateWithUTF8CString(c"name".as_ptr());
                let name_value = jsc::JSValueMakeString(self.raw, error_name_str);
                jsc::JSObjectSetProperty(
                    self.raw,
                    obj,
                    name_key,
                    name_value,
                    jsc::kJSPropertyAttributeDontEnum,
                    ptr::null_mut(),
                );
                jsc::JSStringRelease(name_key);

                error = obj;
            }

            if let Some(code) = code {
                let code_cstr = CString::new(code).unwrap();
                let code_key = jsc::JSStringCreateWithUTF8CString(c"code".as_ptr());
                let code_value_str = jsc::JSStringCreateWithUTF8CString(code_cstr.as_ptr());
                let code_value = jsc::JSValueMakeString(self.raw, code_value_str);
                jsc::JSObjectSetProperty(
                    self.raw,
                    error,
                    code_key,
                    code_value,
                    jsc::kJSPropertyAttributeDontEnum,
                    ptr::null_mut(),
                );
                jsc::JSStringRelease(code_key);
                jsc::JSStringRelease(code_value_str);
            }

            jsc::JSStringRelease(message_str);
            jsc::JSStringRelease(error_name_str);

            JSCValue::from_owned_obj(self.raw, error).with_error()
        }
    }
}
