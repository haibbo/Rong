use crate::{jsc, JSCRuntime, JSCValue};
use rusty_js_core::{JSContextImpl, JSExceptionHandler, JSRuntimeImpl, JSValueImpl};
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

    fn eval(&self, source: rusty_js_core::Source) -> Self::Value {
        let filename = source.name().unwrap_or("eval");
        let code = CString::new(source.code()).unwrap();
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

    fn register_class<JC>(&self) -> Self::Value
    where
        JC: rusty_js_core::JSClass<Self::Value>,
    {
        unsafe {
            let class_name = CString::new(JC::NAME).unwrap();
            let class_name = class_name.as_ptr();
            let class_def = jsc::JSClassDefinition {
                version: 0,
                attributes: 0,
                className: class_name,
                parentClass: ptr::null_mut(),
                staticValues: ptr::null(),
                staticFunctions: ptr::null(),
                initialize: None,
                finalize: Some(crate::class::finalizer::<JC>),
                hasProperty: None,
                getProperty: None,
                setProperty: None,
                deleteProperty: None,
                getPropertyNames: None,
                callAsFunction: Some(crate::class::call_as_function::<JC>),
                callAsConstructor: Some(crate::class::generic_constructor::<JC>),
                hasInstance: Some(crate::class::has_instance),
                convertToType: None,
            };

            let js_class = jsc::JSClassCreate(&class_def);
            let js_class = jsc::JSClassRetain(js_class);
            let constructor = jsc::JSObjectMake(self.raw, js_class, ptr::null_mut());

            // Very Important!
            // It's used to get JSClassRef from constructor's private data, then we can make
            // instance
            // memory is align, so we can set LSB bit to identify it's JSClass
            let classid = js_class as usize | 1;
            jsc::JSObjectSetPrivate(constructor, classid as _);

            let class_name = jsc::JSStringCreateWithUTF8CString(class_name);
            let constructor_ref = jsc::JSStringCreateWithUTF8CString(c"constructor".as_ptr());

            // setup constructor's attribute: name
            let nameproperty = jsc::JSStringCreateWithUTF8CString(c"name".as_ptr());
            let namevalueref = jsc::JSValueMakeString(self.raw, class_name);
            let mut exception: jsc::JSValueRef = std::ptr::null_mut();

            jsc::JSObjectSetProperty(
                self.raw,
                constructor,
                nameproperty,
                namevalueref,
                jsc::kJSPropertyAttributeReadOnly | jsc::kJSPropertyAttributeDontEnum,
                &mut exception,
            );
            jsc::JSStringRelease(nameproperty);

            // define prototype object
            let prototypeobject = jsc::JSObjectMake(self.raw, ptr::null_mut(), ptr::null_mut());
            let prototypeprop = jsc::JSStringCreateWithUTF8CString(c"prototype".as_ptr());
            jsc::JSObjectSetProperty(
                self.raw,
                constructor,
                prototypeprop,
                prototypeobject,
                jsc::kJSPropertyAttributeDontEnum
                    | jsc::kJSPropertyAttributeReadOnly
                    | jsc::kJSPropertyAttributeDontDelete,
                ptr::null_mut(),
            );
            jsc::JSStringRelease(prototypeprop);

            // set prototype.constructor:  constructor
            jsc::JSObjectSetProperty(
                self.raw,
                prototypeobject,
                constructor_ref,
                constructor,
                jsc::kJSPropertyAttributeDontEnum,
                ptr::null_mut(),
            );

            // get global object
            let global = jsc::JSContextGetGlobalObject(self.raw);

            // get Function's Constructor
            let functionname = jsc::JSStringCreateWithUTF8CString(c"Function".as_ptr());
            let functionvalue =
                jsc::JSObjectGetProperty(self.raw, global, functionname, ptr::null_mut());
            jsc::JSStringRelease(functionname);

            // make sure functionvalue is object and Function
            if jsc::JSValueIsObject(self.raw, functionvalue) {
                let functionconstructor =
                    jsc::JSValueToObject(self.raw, functionvalue, ptr::null_mut());

                // set JC::NAME.constructor to Function
                jsc::JSObjectSetProperty(
                    self.raw,
                    constructor,
                    constructor_ref,
                    functionconstructor,
                    jsc::kJSPropertyAttributeDontEnum,
                    ptr::null_mut(),
                );
            }

            // register constructor function to global object
            jsc::JSObjectSetProperty(
                self.raw,
                global,
                class_name,
                constructor,
                jsc::kJSPropertyAttributeNone,
                ptr::null_mut(),
            );
            jsc::JSStringRelease(class_name);
            jsc::JSStringRelease(constructor_ref);

            JSCValue::from_owned_obj(self.raw, constructor)
        }
    }

    fn call(
        &self,
        function: &Self::Value,
        this: Option<Self::Value>,
        argv: Vec<Self::Value>,
    ) -> Self::Value {
        let mut exception: jsc::JSValueRef = std::ptr::null_mut();

        let this_obj = this.map_or_else(
            || unsafe { jsc::JSValueMakeUndefined(self.raw) },
            |v| {
                let raw = *v.as_raw_value();
                raw.cast()
            },
        );

        // Convert argv to raw JSValues
        let args: Vec<jsc::JSValueRef> = argv
            .iter()
            .map(|v| {
                let raw = *v.as_raw_value();
                raw.cast()
            })
            .collect();

        let result = unsafe {
            jsc::JSObjectCallAsFunction(
                self.raw,
                function.as_obj(),
                this_obj as jsc::JSObjectRef,
                args.len(),
                args.as_ptr(),
                &mut exception,
            )
        };

        if !exception.is_null() {
            return JSCValue::from_owned_raw(self.raw, exception).with_exception();
        }

        JSCValue::from_owned_raw(self.raw, result)
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
                JSCValue::from_owned_obj(self.raw, promise),
                JSCValue::from_owned_obj(self.raw, resolve_fn),
                JSCValue::from_owned_obj(self.raw, reject_fn),
            )
        }
    }

    fn compile_to_bytecode(&self, _source: rusty_js_core::Source) -> Option<Vec<u8>> {
        None
    }

    fn run_bytecode(&self, bytes: &[u8]) -> Self::Value {
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

impl JSExceptionHandler for JSCContext {
    fn throw_syntax_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.create_error(message.as_ref())
    }

    fn throw_type_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.create_error(message.as_ref())
    }

    fn throw_reference_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.create_error(message.as_ref())
    }

    fn throw_range_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.create_error(message.as_ref())
    }

    fn throw_error(&self, message: impl AsRef<str>) -> Self::Value {
        self.create_error(message.as_ref())
    }

    fn new_error(&self) -> Self::Value {
        self.create_error("")
    }
}

impl JSCContext {
    fn create_error(&self, message: &str) -> JSCValue {
        unsafe {
            let c_message = CString::new(message).unwrap();
            let js_str = jsc::JSStringCreateWithUTF8CString(c_message.as_ptr());

            let args = [jsc::JSValueMakeString(self.raw, js_str)];
            let exception: *mut jsc::JSValueRef = std::ptr::null_mut();

            let error = jsc::JSObjectMakeError(self.raw, 1, args.as_ptr(), exception);

            jsc::JSStringRelease(js_str);

            if !exception.is_null() {
                JSCValue::from_owned_raw(self.raw, *exception).with_exception()
            } else {
                JSCValue::from_owned_obj(self.raw, error).with_exception()
            }
        }
    }
}
