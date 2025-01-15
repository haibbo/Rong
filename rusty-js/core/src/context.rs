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

impl<C: JSContextImpl> JSContext<C> {
    /// Creates a new JavaScript context that can be safely shared between callbacks and async tasks.
    ///
    /// This function:
    /// 1. Creates a JSContext instance on the heap with a stable address
    /// 2. Stores this address in an opaque data structure for later retrieval
    /// 3. Uses Box::leak to keep the original context alive for callbacks
    ///
    /// The context will be automatically cleaned up when all clones are dropped.
    ///
    /// # Example
    /// ```
    /// let ctx = JSContext::new(&runtime);
    ///
    /// // Can be safely cloned for async tasks
    /// let ctx_clone = ctx.clone();
    /// spawn_local(async move {
    ///     ctx_clone.eval("...").await?;
    /// });
    /// ```
    pub fn new(runtime: &C::Runtime) -> Self {
        // Create the inner context first
        let inner = C::new(runtime);

        // Create the JSContext on the heap to get a stable address
        // This instance will be leaked and cleaned up when the last clone is dropped
        let ctx = Box::new(Self {
            inner: Rc::new(inner),
        });

        // Store the heap address in the opaque data for later retrieval in callbacks
        let opaque = Box::into_raw(ContextOpaque::new(ctx.as_ref() as *const _ as usize));
        ctx.inner.set_opaque::<ContextOpaque<C::Value>>(opaque);

        let leaked_ctx = Box::leak(ctx);
        // Return a clone of the leaked context
        Self {
            inner: leaked_ctx.inner.clone(),
        }
    }

    /// Creates a JSContext from an FFI context pointer.
    ///
    /// This is used in callback scenarios where the JS engine provides a context pointer.
    /// From the JS engine's perspective, contexts created from the mainline and from
    /// callbacks are equivalent since they operate within the same execution context.
    ///
    /// # Safety
    /// - The provided FFI context must be valid and properly aligned
    /// - The caller must ensure the context pointer remains valid for the duration of use
    /// - This should only be called with context pointers obtained from the JS engine
    pub(crate) fn from_ffi(c: &C) -> &Self {
        let data = c.get_opaque::<ContextOpaque<C::Value>>();
        if data.is_null() {
            panic!("[JSContext] opaque is empty");
        } else {
            unsafe {
                let address = (*data).address;
                std::mem::transmute::<usize, &JSContext<C>>(address)
            }
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

#[cfg(feature = "debug_log")]
macro_rules! context_log {
    ($op:expr, $count:expr) => {
        println!("[JSContext] {} ref count: {}", $op, $count)
    };
    ($($arg:tt)*) => {
        println!("[JSContext] {}", format!($($arg)*))
    };
}
#[cfg(not(feature = "debug_log"))]
macro_rules! context_log {
    ($op:expr, $count:expr) => {};
    ($($arg:tt)*) => {};
}

/// Container to hold the context-specific data for a JSContext.
///
/// # Fields
/// - `registry`: A pointer to a RefCell containing a HashMap that maps TypeId to type that implements JSValueImpl
/// - `ref_count`: An AtomicUsize to track the reference count of the context
/// - `address`: Address of JSContext used to build JSContext from callback case
struct ContextOpaque<V: JSValueImpl> {
    registry: RefCell<HashMap<TypeId, V>>,
    ref_count: AtomicUsize,
    address: usize,
}

impl<V: JSValueImpl> ContextOpaque<V> {
    fn new(address: usize) -> Box<Self> {
        Box::new(Self {
            registry: RefCell::new(HashMap::new()),
            ref_count: AtomicUsize::new(1),
            address,
        })
    }

    fn inc_ref(&self) {
        #[allow(unused_variables)]
        let prev_count = self.ref_count.fetch_add(1, Ordering::SeqCst);
        context_log!("increment", prev_count + 1);
    }

    fn dec_ref(&self) -> bool {
        let prev_count = self.ref_count.fetch_sub(1, Ordering::SeqCst);
        match prev_count {
            n if n > 1 => {
                context_log!("decrement", n - 1);
                false
            }
            1 => {
                context_log!("decrement to zero");
                true
            }
            _ => false,
        }
    }
}

impl<C: JSContextImpl> Drop for JSContext<C> {
    fn drop(&mut self) {
        let data = self.inner.get_opaque::<ContextOpaque<C::Value>>();

        if !data.is_null() {
            unsafe {
                // Check if this is the last reference
                if (*data).dec_ref() {
                    context_log!("cleanup context and resources");

                    // Get all the pointers we need before any cleanup
                    let registry = &(*data).registry;
                    let ctx_ptr = (*data).address as *mut JSContext<C>;

                    // Clear the registry first
                    registry.borrow_mut().clear();

                    // Clean up the leaked JSContext
                    let _ = Box::from_raw(ctx_ptr);

                    // Finally clean up the ContextOpaque
                    let _ = Box::from_raw(data);
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
            }
        }

        Self {
            inner: self.inner.clone(),
        }
    }
}
