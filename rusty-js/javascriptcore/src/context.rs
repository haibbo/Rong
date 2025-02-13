use crate::{jsc, JSCRuntime, JSCValue};
use rusty_js_core::{JSContextImpl, JSExceptionHandler, JSRuntimeImpl, JSValueImpl, RustyJSError};
use std::ffi::{c_char, CString};
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

    /// Bug:
    ///
    /// It is not possible to use JS subclassing with objects created from a class
    /// definition that sets callAsConstructor by default. The callAsConstructor's
    /// constructor is not changed to extended class constructor.
    ///
    /// Subclassing is supported via the JSObjectMakeConstructor function, but it has
    /// disadvantages:
    /// - can not set private data to constructor object. WeakMap is alternative solution.
    /// - typeof constructor is 'object' not 'function'(nodejs, bun, quickjs etc)
    ///
    /// Because the crate is specially desigend for mini-program, we shoud not avoid
    /// to use subclass/extend-class, in other word, directly extend feature at native
    /// side.
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
            let constructor_name = jsc::JSStringCreateWithUTF8CString(c"constructor".as_ptr());
            let proto_name = jsc::JSStringCreateWithUTF8CString(c"prototype".as_ptr());

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

            // Create prototype object
            let prototypeobject = jsc::JSObjectMake(self.raw, ptr::null_mut(), ptr::null_mut());

            // Set JC::NAME.prototype
            jsc::JSObjectSetProperty(
                self.raw,
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
                self.raw,
                prototypeobject,
                constructor_name,
                constructor,
                jsc::kJSPropertyAttributeDontEnum,
                ptr::null_mut(),
            );

            // Get Function constructor using helper function
            let functionconstructor = get_constructor(self.raw, c"Function".as_ptr());

            // Set JC::NAME.constructor to Function
            jsc::JSObjectSetProperty(
                self.raw,
                constructor,
                constructor_name,
                functionconstructor,
                jsc::kJSPropertyAttributeDontEnum,
                ptr::null_mut(),
            );

            // register constructor function to global object
            let global = jsc::JSContextGetGlobalObject(self.raw);
            jsc::JSObjectSetProperty(
                self.raw,
                global,
                class_name,
                constructor,
                jsc::kJSPropertyAttributeNone,
                ptr::null_mut(),
            );
            jsc::JSStringRelease(class_name);
            jsc::JSStringRelease(constructor_name);
            jsc::JSStringRelease(proto_name);

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

    fn compile_to_bytecode(&self, _source: rusty_js_core::Source) -> Result<Vec<u8>, RustyJSError> {
        Err(RustyJSError::NotSupportByteCode)
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

impl JSExceptionHandler for JSCContext {
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
        let args = [];
        unsafe {
            let error = jsc::JSObjectMakeError(self.raw, 0, args.as_ptr(), ptr::null_mut());
            JSCValue::from_owned_obj(self.raw, error).with_error()
        }
    }
}

impl JSCContext {
    /// Helper function to throw an error with a specific name
    pub(crate) fn throw_error_with_name(
        &self,
        error_name: &str,
        message: impl AsRef<str>,
    ) -> JSCValue {
        #[cfg(debug_assertions)]
        println!("{}: {}", error_name, message.as_ref());

        let message_cstr = CString::new(message.as_ref()).unwrap();
        let error_name_cstr = CString::new(error_name).unwrap();

        unsafe {
            let message_str = jsc::JSStringCreateWithUTF8CString(message_cstr.as_ptr());
            let error_name_str = jsc::JSStringCreateWithUTF8CString(error_name_cstr.as_ptr());
            let proto_key = jsc::JSStringCreateWithUTF8CString(c"prototype".as_ptr());
            let name_key = jsc::JSStringCreateWithUTF8CString(c"name".as_ptr());

            // Get constructor
            let global = jsc::JSContextGetGlobalObject(self.raw);
            let error_constructor =
                jsc::JSObjectGetProperty(self.raw, global, error_name_str, ptr::null_mut());

            let error = if !error_constructor.is_null() {
                let message_value = jsc::JSValueMakeString(self.raw, message_str);
                let args = [message_value];
                let error = jsc::JSObjectCallAsConstructor(
                    self.raw,
                    error_constructor as jsc::JSObjectRef,
                    1,
                    args.as_ptr(),
                    ptr::null_mut(),
                );

                if !error.is_null() {
                    let error_proto = jsc::JSObjectGetProperty(
                        self.raw,
                        error_constructor as jsc::JSObjectRef,
                        proto_key,
                        ptr::null_mut(),
                    );
                    if !error_proto.is_null() {
                        jsc::JSObjectSetPrototype(self.raw, error as jsc::JSObjectRef, error_proto);
                    }

                    let name_value = jsc::JSValueMakeString(self.raw, error_name_str);
                    jsc::JSObjectSetProperty(
                        self.raw,
                        error as jsc::JSObjectRef,
                        name_key,
                        name_value,
                        jsc::kJSPropertyAttributeDontEnum,
                        ptr::null_mut(),
                    );
                }
                error
            } else {
                ptr::null_mut()
            };

            jsc::JSStringRelease(message_str);
            jsc::JSStringRelease(error_name_str);
            jsc::JSStringRelease(proto_key);
            jsc::JSStringRelease(name_key);

            if error.is_null() {
                let generic_message = format!("{}: {}", error_name, message.as_ref());
                let generic_cstr = CString::new(generic_message).unwrap();
                let generic_str = jsc::JSStringCreateWithUTF8CString(generic_cstr.as_ptr());

                let args = [jsc::JSValueMakeString(self.raw, generic_str)];
                let error = jsc::JSObjectMakeError(
                    self.raw,
                    1,
                    args.as_ptr(), // Pass pointer to array
                    ptr::null_mut(),
                );
                jsc::JSStringRelease(generic_str);
                JSCValue::from_owned_raw(self.raw, error).with_exception()
            } else {
                JSCValue::from_owned_raw(self.raw, error).with_exception()
            }
        }
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
