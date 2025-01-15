use crate::{
    source::Source, ClassSetup, FromJSValue, JSClass, JSObject, JSObjectOps, JSResult,
    JSRuntimeImpl, JSValue, JSValueImpl, RustyJSError,
};
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

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
    fn new(runtime: &Self::Runtime) -> Self;

    /// Get the opaque pointer stored in the context
    ///
    /// # Safety
    /// - The caller must ensure the pointer is valid and properly aligned for type T
    /// - The pointer must not be used after the context is dropped
    fn get_opaque<T>(&self) -> *mut T;

    /// Set the opaque pointer in the context
    ///
    /// # Safety
    /// - The caller must ensure the pointer is valid and properly aligned for type T
    /// - The caller must ensure proper cleanup of the pointer when no longer needed
    fn set_opaque<T>(&self, opaque: *mut T);

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
    /// New JSContext
    pub fn new(runtime: &C::Runtime) -> Self {
        let ctx = C::new(runtime);
        ctx.set_opaque::<ContextOpaque<C::Value>>(Box::into_raw(ContextOpaque::new()));
        Self {
            inner: Rc::new(ctx),
        }
    }

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

        if let Some(registry) = self.get_class_registry() {
            registry
                .borrow_mut()
                .insert(TypeId::of::<JC>(), constructor.clone());
        }

        let obj = self.global();
        let constructor = JSValue::new(self, constructor);
        JC::class_setup(&ClassSetup::new(constructor.clone().into(), self));
        obj.set(JC::NAME, constructor);
    }

    /// Get class registry from context
    pub(crate) fn get_class_registry(&self) -> Option<&RefCell<HashMap<TypeId, C::Value>>> {
        let data = self.inner.get_opaque::<ContextOpaque<C::Value>>();
        if data.is_null() {
            None
        } else {
            unsafe { Some(&(*data).registry) }
        }
    }
}

#[cfg(feature = "ref_count_tracking")]
macro_rules! ref_count_println {
    ($($arg:tt)*) => (println!($($arg)*));
}

#[cfg(not(feature = "ref_count_tracking"))]
macro_rules! ref_count_println {
    ($($arg:tt)*) => {};
}

/// Container to hold the context-specific data for a JSContext.
///
/// # Fields
/// - `registry`: A pointer to a RefCell containing a HashMap that maps TypeId to type that implements JSValueImpl
/// - `ref_count`: An AtomicUsize to track the reference count of the context
struct ContextOpaque<V: JSValueImpl> {
    registry: RefCell<HashMap<TypeId, V>>,
    ref_count: AtomicUsize,
}

impl<V: JSValueImpl> ContextOpaque<V> {
    fn new() -> Box<Self> {
        // Creates a new class registry
        let registry = RefCell::new(HashMap::new());

        Box::new(Self {
            registry,
            ref_count: AtomicUsize::new(1),
        })
    }

    fn inc_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
    }

    fn dec_ref(&self) -> usize {
        self.ref_count.fetch_sub(1, Ordering::SeqCst)
    }
}

impl<C: JSContextImpl> Drop for JSContext<C> {
    fn drop(&mut self) {
        let data = self.inner.get_opaque::<ContextOpaque<C::Value>>();

        if !data.is_null() {
            unsafe {
                // If it's the last reference, clean up registry and ContextData
                if (*data).dec_ref() == 1 {
                    ref_count_println!("free JSContext on last drop (ref_count: 0)");

                    // Get the registry from the context's opaque data
                    let registry = &(*data).registry;
                    // Clear the contents fristly
                    registry.borrow_mut().clear();

                    let _ = Box::from_raw(data);
                } else {
                    ref_count_println!(
                        "skip free JSContext on drop (ref_count: {})",
                        (*data).ref_count.load(Ordering::SeqCst)
                    );
                }
            }
        }
    }
}

impl<C: JSContextImpl> Clone for JSContext<C> {
    fn clone(&self) -> Self {
        let data = self.inner.get_opaque::<ContextOpaque<C::Value>>();
        if !data.is_null() {
            unsafe {
                (*data).inc_ref();
                ref_count_println!(
                    "increment JSContext ref on clone (ref_count: {})",
                    (*data).ref_count.load(Ordering::SeqCst)
                );
            }
        }

        Self {
            inner: self.inner.clone(),
        }
    }
}
