use crate::{ArkJSRuntime, ArkJSValue, arkjs};
use rong_core::{
    JSClass, JSContextImpl, JSExceptionHandler, JSRuntimeImpl, JSValueImpl, RongJSError,
};
use std::ffi::CString;
use std::ptr;

pub struct ArkJSContext {
    raw: arkjs::JSVM_Env,
    vm: arkjs::JSVM_VM,
}

impl JSContextImpl for ArkJSContext {
    type RawContext = arkjs::JSVM_Env;
    type Runtime = ArkJSRuntime;
    type Value = ArkJSValue;

    fn new(runtime: &Self::Runtime) -> Self {
        let mut env: arkjs::JSVM_Env = ptr::null_mut();

        unsafe {
            let status = arkjs::OH_JSVM_CreateEnv(runtime.to_raw(), 0, ptr::null(), &mut env);
            if status != arkjs::JSVM_Status_JSVM_OK {
                panic!("Failed to create Ark JS environment: {:?}", status);
            }
        }

        Self {
            raw: env,
            vm: runtime.to_raw(),
        }
    }

    fn as_raw(&self) -> &Self::RawContext {
        &self.raw
    }

    fn context_id(ctx: &Self::RawContext) -> usize {
        *ctx as *const _ as usize
    }

    fn from_borrowed_raw(ctx: Self::RawContext) -> Self {
        // Note: This is a simplified implementation
        // In practice, we might need to track the VM reference
        Self {
            raw: ctx,
            vm: ptr::null_mut(), // This should be properly managed
        }
    }

    fn eval(&self, source: rong_core::Source) -> Self::Value {
        let code = source.code();

        unsafe {
            let mut result: arkjs::JSVM_Value = ptr::null_mut();
            let code_cstr = CString::new(code).unwrap();

            // First create a string value for the script
            let mut script_value: arkjs::JSVM_Value = ptr::null_mut();
            let status = arkjs::OH_JSVM_CreateStringUtf8(
                self.raw,
                code_cstr.as_ptr(),
                code.len(),
                &mut script_value,
            );

            if status != arkjs::JSVM_Status_JSVM_OK {
                return Self::Value::create_undefined(self);
            }

            // Compile the script
            let mut script: arkjs::JSVM_Script = ptr::null_mut();
            let mut cache_rejected = false;
            let status = arkjs::OH_JSVM_CompileScript(
                self.raw,
                script_value,
                ptr::null(),
                0,
                true,
                &mut cache_rejected,
                &mut script,
            );

            if status != arkjs::JSVM_Status_JSVM_OK {
                return Self::Value::create_undefined(self);
            }

            // Run the script
            let status = arkjs::OH_JSVM_RunScript(self.raw, script, &mut result);

            if status != arkjs::JSVM_Status_JSVM_OK {
                return Self::Value::create_undefined(self);
            }

            ArkJSValue::from_owned_raw(self.raw, result)
        }
    }

    fn global(&self) -> Self::Value {
        unsafe {
            let mut global: arkjs::JSVM_Value = ptr::null_mut();
            let status = arkjs::OH_JSVM_GetGlobal(self.raw, &mut global);

            if status != arkjs::JSVM_Status_JSVM_OK {
                panic!("Failed to get global object: {:?}", status);
            }

            ArkJSValue::from_owned_raw(self.raw, global)
        }
    }

    fn register_class<JC>(&self) -> Self::Value
    where
        JC: JSClass<Self::Value>,
    {
        crate::class::register_class_internal::<JC>(self, JC::NAME)
    }

    fn call(
        &self,
        function: &Self::Value,
        this: Self::Value,
        argv: Vec<Self::Value>,
    ) -> Self::Value {
        unsafe {
            let mut result: arkjs::JSVM_Value = ptr::null_mut();

            // Convert argv to raw JSVM_Values
            let args: Vec<arkjs::JSVM_Value> = argv.iter().map(|v| *v.as_raw_value()).collect();

            let status = arkjs::OH_JSVM_CallFunction(
                self.raw,
                *this.as_raw_value(),
                *function.as_raw_value(),
                args.len(),
                args.as_ptr(),
                &mut result,
            );

            if status != arkjs::JSVM_Status_JSVM_OK {
                let mut exception: arkjs::JSVM_Value = ptr::null_mut();
                arkjs::OH_JSVM_GetAndClearLastException(self.raw, &mut exception);
                return ArkJSValue::from_owned_raw(self.raw, exception).with_exception();
            }

            ArkJSValue::from_owned_raw(self.raw, result)
        }
    }

    fn promise(&self) -> (Self::Value, Self::Value, Self::Value) {
        unsafe {
            let mut deferred: arkjs::JSVM_Deferred = ptr::null_mut();
            let mut promise: arkjs::JSVM_Value = ptr::null_mut();

            let status = arkjs::OH_JSVM_CreatePromise(self.raw, &mut deferred, &mut promise);

            if status != arkjs::JSVM_Status_JSVM_OK {
                let undefined = Self::Value::create_undefined(self);
                return (undefined.clone(), undefined.clone(), undefined);
            }

            // Create resolve function that wraps OH_JSVM_ResolveDeferred
            let resolve_func = create_resolve_function(self, deferred);

            // Create reject function that wraps OH_JSVM_RejectDeferred
            let reject_func = create_reject_function(self, deferred);

            (
                ArkJSValue::from_owned_raw(self.raw, promise),
                resolve_func,
                reject_func,
            )
        }
    }

    fn compile_to_bytecode(&self, _source: rong_core::Source) -> Result<Vec<u8>, RongJSError> {
        // ArkJS bytecode compilation APIs are not available in current bindings
        // Return error to indicate bytecode compilation is not supported
        Err(RongJSError::NotSupportByteCode)
    }

    fn run_bytecode(&self, _bytes: &[u8]) -> Self::Value {
        // ArkJS bytecode execution APIs are not available in current bindings
        // Return undefined as this operation is not supported
        ArkJSValue::create_undefined(self)
    }
}

impl ArkJSContext {
    pub(crate) fn to_raw(&self) -> arkjs::JSVM_Env {
        self.raw
    }
}

impl Drop for ArkJSContext {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                arkjs::OH_JSVM_DestroyEnv(self.raw);
            }
        }
    }
}

impl Clone for ArkJSContext {
    fn clone(&self) -> Self {
        // Note: This is a simplified implementation
        // In practice, we might need to create a new environment or reference count
        Self {
            raw: self.raw,
            vm: self.vm,
        }
    }
}

impl JSExceptionHandler for ArkJSContext {
    fn throw_syntax_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_error_with_name("SyntaxError", message)
    }

    fn throw_type_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_error_with_name("TypeError", message)
    }

    fn throw_reference_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_error_with_name("ReferenceError", message)
    }

    fn throw_range_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_error_with_name("RangeError", message)
    }

    fn throw_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.throw_error_with_name("Error", message)
    }

    fn new_error(&self) -> Self::Value {
        unsafe {
            let mut error: arkjs::JSVM_Value = ptr::null_mut();
            let mut message: arkjs::JSVM_Value = ptr::null_mut();
            arkjs::OH_JSVM_CreateStringUtf8(self.raw, b"Error\0".as_ptr(), 5, &mut message);

            let status = arkjs::OH_JSVM_CreateError(self.raw, ptr::null_mut(), message, &mut error);

            if status == arkjs::JSVM_Status_JSVM_OK {
                ArkJSValue::from_owned_raw(self.raw, error).with_error()
            } else {
                Self::Value::create_undefined(self)
            }
        }
    }

    fn throw(&self, value: Self::Value) -> Self::Value {
        value.with_exception()
    }
}

impl ArkJSContext {
    pub(crate) fn throw_error_with_name(
        &self,
        error_name: &str,
        message: impl AsRef<str>,
    ) -> ArkJSValue {
        unsafe {
            let mut error: arkjs::JSVM_Value = ptr::null_mut();
            let message_cstr = CString::new(message.as_ref()).unwrap();
            let _error_name_cstr = CString::new(error_name).unwrap();

            let mut message_value: arkjs::JSVM_Value = ptr::null_mut();
            arkjs::OH_JSVM_CreateStringUtf8(
                self.raw,
                message_cstr.as_ptr(),
                message.as_ref().len(),
                &mut message_value,
            );

            let status =
                arkjs::OH_JSVM_CreateError(self.raw, ptr::null_mut(), message_value, &mut error);

            if status == arkjs::JSVM_Status_JSVM_OK {
                // Try to set the error name if the API supports it
                // This might need adjustment based on actual Ark JS API
                ArkJSValue::from_owned_raw(self.raw, error).with_exception()
            } else {
                // Fallback to a generic error
                ArkJSValue::create_undefined(self).with_exception()
            }
        }
    }
}

// Helper function to create a resolve function for a deferred promise
fn create_resolve_function(ctx: &ArkJSContext, deferred: arkjs::JSVM_Deferred) -> ArkJSValue {
    unsafe {
        let mut resolve_func: arkjs::JSVM_Value = ptr::null_mut();

        // Create a function that will call OH_JSVM_ResolveDeferred
        let callback_struct = arkjs::JSVM_CallbackStruct {
            callback: Some(resolve_callback),
            data: deferred as *mut std::ffi::c_void,
        };

        let name_cstr = CString::new("resolve").unwrap();
        let status = arkjs::OH_JSVM_CreateFunction(
            ctx.to_raw(),
            name_cstr.as_ptr(),
            7, // length of "resolve"
            &callback_struct as *const _ as *mut _,
            &mut resolve_func,
        );

        if status == arkjs::JSVM_Status_JSVM_OK {
            ArkJSValue::from_owned_raw(ctx.to_raw(), resolve_func)
        } else {
            ArkJSValue::create_undefined(ctx)
        }
    }
}

// Helper function to create a reject function for a deferred promise
fn create_reject_function(ctx: &ArkJSContext, deferred: arkjs::JSVM_Deferred) -> ArkJSValue {
    unsafe {
        let mut reject_func: arkjs::JSVM_Value = ptr::null_mut();

        // Create a function that will call OH_JSVM_RejectDeferred
        let callback_struct = arkjs::JSVM_CallbackStruct {
            callback: Some(reject_callback),
            data: deferred as *mut std::ffi::c_void,
        };

        let name_cstr = CString::new("reject").unwrap();
        let status = arkjs::OH_JSVM_CreateFunction(
            ctx.to_raw(),
            name_cstr.as_ptr(),
            6, // length of "reject"
            &callback_struct as *const _ as *mut _,
            &mut reject_func,
        );

        if status == arkjs::JSVM_Status_JSVM_OK {
            ArkJSValue::from_owned_raw(ctx.to_raw(), reject_func)
        } else {
            ArkJSValue::create_undefined(ctx)
        }
    }
}

// Callback function for resolve
unsafe extern "C" fn resolve_callback(
    env: arkjs::JSVM_Env,
    info: arkjs::JSVM_CallbackInfo,
) -> arkjs::JSVM_Value {
    unsafe {
        let mut argc: usize = 1;
        let mut argv: [arkjs::JSVM_Value; 1] = [ptr::null_mut()];
        let mut this_arg: arkjs::JSVM_Value = ptr::null_mut();
        let mut data: *mut std::ffi::c_void = ptr::null_mut();

        let status = arkjs::OH_JSVM_GetCbInfo(
            env,
            info,
            &mut argc,
            argv.as_mut_ptr(),
            &mut this_arg,
            &mut data,
        );

        if status == arkjs::JSVM_Status_JSVM_OK && !data.is_null() {
            let deferred = data as arkjs::JSVM_Deferred;
            let resolution = if argc > 0 { argv[0] } else { ptr::null_mut() };

            // Resolve the deferred promise
            arkjs::OH_JSVM_ResolveDeferred(env, deferred, resolution);
        }

        // Return undefined
        let mut undefined: arkjs::JSVM_Value = ptr::null_mut();
        arkjs::OH_JSVM_GetUndefined(env, &mut undefined);
        undefined
    }
}

// Callback function for reject
unsafe extern "C" fn reject_callback(
    env: arkjs::JSVM_Env,
    info: arkjs::JSVM_CallbackInfo,
) -> arkjs::JSVM_Value {
    unsafe {
        let mut argc: usize = 1;
        let mut argv: [arkjs::JSVM_Value; 1] = [ptr::null_mut()];
        let mut this_arg: arkjs::JSVM_Value = ptr::null_mut();
        let mut data: *mut std::ffi::c_void = ptr::null_mut();

        let status = arkjs::OH_JSVM_GetCbInfo(
            env,
            info,
            &mut argc,
            argv.as_mut_ptr(),
            &mut this_arg,
            &mut data,
        );

        if status == arkjs::JSVM_Status_JSVM_OK && !data.is_null() {
            let deferred = data as arkjs::JSVM_Deferred;
            let rejection = if argc > 0 { argv[0] } else { ptr::null_mut() };

            // Reject the deferred promise
            arkjs::OH_JSVM_RejectDeferred(env, deferred, rejection);
        }

        // Return undefined
        let mut undefined: arkjs::JSVM_Value = ptr::null_mut();
        arkjs::OH_JSVM_GetUndefined(env, &mut undefined);
        undefined
    }
}

