use crate::{
    source::Source, ClassSetup, FromJSValue, JSClass, JSObject, JSObjectOps, JSResult,
    JSRuntimeImpl, JSValue, JSValueImpl, RustyJSError,
};
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// JSContextImpl represents a JavaScript context
///
/// # Safety
/// The implementation must ensure that:
/// 1. Value type implements Drop to properly clean up resources
/// 2. Context type implements Drop if it holds any resources that need cleanup
pub trait JSContextImpl: Clone {
    /// the JS engine specific type of JavaScript Context
    type FfiContext: Copy;
    type Runtime: JSRuntimeImpl<Context = Self>;
    type Value: JSValueImpl<Context = Self>;

    /// Creates a new JavaScript context
    ///
    /// # Arguments
    /// * `runtime` - The JavaScript runtime to associate with this context
    /// * `registry` - Raw pointer to the class registry (created by JSContext::new_class_registry)
    ///
    /// # Safety
    /// The implementation must ensure:
    /// - The registry pointer is stored in the context
    /// - The registry is properly cleaned up when the context is dropped by calling free_class_registry
    /// - The registry pointer is invalid after the context is dropped
    fn new(runtime: &Self::Runtime, registry: *mut RefCell<HashMap<TypeId, Self::Value>>) -> Self;

    /// FfiContext implements Copy
    fn to_ffi(&self) -> Self::FfiContext;

    /// the implementation need to make sure it has the ownship, like as new method
    /// generally, it should increase referen count of FFI Context
    fn from_ffi(ctx: Self::FfiContext) -> Self;

    /// Evaluate JavaScript code
    fn eval(&self, source: Source) -> Self::Value;

    /// Get global object
    fn global(&self) -> Self::Value;

    /// Register class for rust type
    fn register_class<JC>(&self) -> Self::Value
    where
        JC: JSClass<Self::Value>;

    /// Get class registry from context
    fn get_class_registry(&self) -> Option<&RefCell<HashMap<TypeId, Self::Value>>>;

    /// Frees a class registry safely
    ///
    /// # Safety
    /// - The pointer must have been created by JSContext::new_class_registry
    /// - The pointer must not be null
    /// - The pointer must not be used after this call
    /// - This function must be called exactly once for each registry
    unsafe fn free_class_registry(registry: *mut RefCell<HashMap<TypeId, Self::Value>>) {
        debug_assert!(
            !registry.is_null(),
            "free_class_registry called with null pointer"
        );
        if !registry.is_null() {
            // Clear the contents first to ensure proper cleanup of any values
            (*registry).borrow_mut().clear();
            // Then drop the Box to free the memory
            let _ = Box::from_raw(registry);
        }
    }

    /// Calls a JavaScript function with the specified `this` value and arguments.
    ///
    /// # Arguments
    ///
    /// * `function` - The JavaScript function to call
    /// * `this` - Optional `this` value to use when calling the function
    /// * `argv` - Vector of arguments to pass to the function
    ///
    /// # Returns
    ///
    /// Returns the result of the function call as a JavaScript value
    fn call(
        &self,
        function: &Self::Value,
        this: Option<Self::Value>,
        argv: Vec<Self::Value>,
    ) -> Self::Value;

    /// Creates a new JavaScript Promise and returns a tuple containing:
    /// - The Promise object
    /// - The resolve function to fulfill the promise
    /// - The reject function to reject the promise
    fn promise(&self) -> (Self::Value, Self::Value, Self::Value);
}

pub trait JSFfiContext {
    type FfiContext;
}

#[derive(Clone)]
pub struct JSContext<C: JSContextImpl> {
    inner: Rc<C>,
}

impl<C: JSContextImpl> AsRef<C> for JSContext<C> {
    fn as_ref(&self) -> &C {
        &self.inner
    }
}

impl<C: JSContextImpl> From<C> for JSContext<C> {
    fn from(c: C) -> Self {
        Self { inner: Rc::new(c) }
    }
}

impl<C: JSContextImpl> JSContext<C> {
    /// eval javascript
    pub fn eval<T>(&self, source: Source) -> JSResult<T>
    where
        C::Value: JSObjectOps,
        T: FromJSValue<C::Value>,
    {
        let raw = self.inner.eval(source);
        let result = JSValue::new(self, raw);

        result.is_exception().map_or_else(
            || T::from_js_value(&self.inner, result.into_inner()),
            |exception| Err(RustyJSError::Exception(exception.into_error())),
        )
    }

    /// get global object
    pub fn global(&self) -> JSObject<C::Value> {
        let raw = self.inner.global();
        JSValue::new(self, raw).into()
    }

    /// Register a JavaScript class for a Rust type.
    ///
    /// This function registers a JavaScript class constructor in the global object
    /// and stores it in the context's class registry. The class can then be used
    /// to create instances in JavaScript.
    ///
    /// ```rust
    /// context.register_class::<MyClass>();
    /// ```
    pub fn register_class<JC>(&self)
    where
        JC: JSClass<C::Value>,
        C::Value: JSObjectOps,
    {
        let constructor = self.inner.register_class::<JC>();

        if let Some(registry) = self.inner.get_class_registry() {
            registry
                .borrow_mut()
                .insert(TypeId::of::<JC>(), constructor.clone());
        }

        let obj = self.global();
        let constructor = JSValue::new(self, constructor);
        JC::class_setup(&ClassSetup::new(constructor.clone().into(), self));
        obj.set(JC::NAME, constructor);
    }

    /// Creates a new class registry
    ///
    /// # Returns
    /// A raw pointer to a RefCell<HashMap> that will be stored in the context.
    /// The context implementation is responsible for calling JSContextImpl::free_class_registry
    /// to properly clean up this memory.
    pub(crate) fn new_class_registry() -> *mut RefCell<HashMap<TypeId, C::Value>> {
        let registry: RefCell<HashMap<TypeId, C::Value>> = RefCell::new(HashMap::new());
        Box::into_raw(Box::new(registry))
    }
}
