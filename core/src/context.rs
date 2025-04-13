use crate::{
    source::{Source, SourceKind},
    ClassSetup, FromJSValue, JSClass, JSException, JSObject, JSObjectOps, JSResult, JSRuntimeImpl,
    JSTypeOf, JSValue, JSValueImpl, Promise, RustyJSError,
};
use crate::{JSRuntime, JSValueMapper};
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::sync::{LazyLock, RwLock};

/// JSContextImpl represents a JavaScript context
///
/// # Safety
/// The implementation must ensure that:
/// 1. Value type implements Drop to properly clean up resources
/// 2. Context type implements Drop if it holds any resources that need cleanup
pub trait JSContextImpl {
    /// The JavaScript engine's native context type.
    ///
    /// This represents the raw, engine-specific context handle that is used internally
    type RawContext;

    /// The runtime type associated with this context.
    ///
    /// This specifies the JavaScript runtime implementation that this context belongs to.
    /// The runtime must implement JSRuntimeImpl and have its Context type set to Self.
    type Runtime: JSRuntimeImpl<Context = Self>;

    /// The JavaScript value type associated with this context.
    ///
    /// This specifies the type used to represent JavaScript values within this context.
    /// The value type must implement JSValueImpl and have its Context type set to Self.
    type Value: JSValueImpl<Context = Self>;

    /// Creates a new JavaScript context
    fn new(runtime: &Self::Runtime) -> Self;

    /// Converts the context to its FFI representation
    fn as_raw(&self) -> &Self::RawContext;

    /// Returns a unique identifier for the context that can be used as a key in CTX_OPAQUE
    ///
    /// This identifier must be:
    /// - Unique per context instance
    /// - Stable for the lifetime of the context
    /// - Suitable for use as a HashMap key
    ///
    /// # Returns
    /// A usize value that uniquely identifies this context instance
    fn context_id(ctx: &Self::RawContext) -> usize;

    /// the implementation need to make sure it has the ownship, like as new method
    /// generally, it should increase referen count of FFI Context
    fn from_borrowed_raw(ctx: Self::RawContext) -> Self;

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
        this: Self::Value,
        argv: Vec<Self::Value>,
    ) -> Self::Value;

    /// Creates a new JavaScript Promise and returns a tuple containing:
    /// - The Promise object
    /// - The resolve function to fulfill the promise
    /// - The reject function to reject the promise
    fn promise(&self) -> (Self::Value, Self::Value, Self::Value);

    /// Compiles JavaScript source code into bytecode format
    ///
    /// # Arguments
    /// * `source` - The JavaScript source code to compile
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - The compiled bytecode as bytes if compilation succeeds
    /// * `Err(RustyJSError)` - If compilation fails with one of these errors:
    ///   - `RustyJSError::CompileToByteErr`: General compilation error
    ///   - `RustyJSError::NotSupportByteCode`: Bytecode compilation not supported by runtime
    fn compile_to_bytecode(&self, source: Source) -> Result<Vec<u8>, RustyJSError>;

    /// Executes previously compiled bytecode
    ///
    /// # Arguments
    /// * `bytes` - The bytecode bytes to execute
    ///
    /// # Returns
    /// The result of executing the bytecode as a JavaScript value
    fn run_bytecode(&self, bytes: &[u8]) -> Self::Value;
}

pub trait JSRawContext {
    type RawContext;
}

pub struct JSContext<C: JSContextImpl> {
    rc: Rc<JSContextInner<C>>,
}

struct JSContextInner<C: JSContextImpl> {
    inner: C,
    runtime: JSRuntime<C::Runtime>,
    rong: C::Value,
}

impl<C: JSContextImpl> AsRef<C> for JSContext<C> {
    fn as_ref(&self) -> &C {
        &self.rc.inner
    }
}

impl<C: JSContextImpl> JSContext<C> {
    /// Creates a new JavaScript context.
    ///
    /// This function:
    /// 1. Creates a JSContext instance with proper internal structure
    /// 2. Stores the context address in an opaque data structure for FFI callbacks
    /// 3. Sets up weak references to prevent memory leaks
    ///
    /// The context can be safely shared between callbacks and async tasks.
    ///
    /// # Safety
    /// - The context must be dropped on the same thread it was created on
    /// - The runtime must outlive the context
    ///
    /// # Example
    /// ```
    /// let rt = RustyJS::runtime();
    /// let ctx = JSContext::new(&rt);
    ///
    /// // Can be safely cloned for async tasks
    /// let ctx_clone = ctx.clone();
    /// ctx.spawn_local(async move {
    ///     ctx_clone.eval_async("...").await?;
    /// });
    /// ```
    pub(crate) fn new(runtime: &JSRuntime<C::Runtime>) -> Self
    where
        C::Value: JSObjectOps,
    {
        let raw_ctx = C::new(&runtime.inner);
        let rong = C::Value::new_object(&raw_ctx);

        let inner = JSContextInner {
            inner: raw_ctx,
            runtime: runtime.clone(),
            rong,
        };

        let ctx = JSContext { rc: Rc::new(inner) };
        let weak = Rc::downgrade(&ctx.rc);

        // save stale address to opaque
        let opaque = ContextOpaque::<C::Value>::new(weak);
        let raw_ctx = ctx.rc.inner.as_raw();
        let key = C::context_id(raw_ctx);
        CTX_OPAQUE
            .write()
            .unwrap()
            .insert(key, Box::into_raw(opaque) as usize);

        ctx
    }

    /// return global Dainty object
    pub fn rong(&self) -> JSObject<C::Value> {
        let value = JSValue::from_raw(self, self.rc.as_ref().rong.clone());
        value.into()
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
    /// - The returned reference must not outlive the original context
    ///
    /// # Example
    /// ```rust
    /// // In a callback from JS engine:
    /// unsafe {
    ///     let ctx = JSContext::from_borrowed_raw_ptr(ffi_ctx);
    ///     // Use ctx for the duration of the callback
    /// }
    /// ```
    pub(crate) fn from_borrowed_raw_ptr(ptr: &C::RawContext) -> Self {
        let data = Self::_get_opaque(ptr);
        if data.is_null() {
            panic!("[JSContext] opaque is empty");
        } else {
            let ctx_inner = unsafe { &(*data).ctx_inner };
            if let Some(ctx) = ctx_inner.upgrade() {
                Self { rc: ctx }
            } else {
                panic!("[JSContext] context has been dropped");
            }
        }
    }

    /// Evaluate JavaScript code and return the result
    ///
    /// # Arguments
    /// * `source` - The JavaScript source code to evaluate
    ///
    /// # Returns
    /// * `Ok(T)` - The result of the evaluation if successful
    /// * `Err(RustyJSError)` - If evaluation fails or throws an exception
    ///
    /// # Examples
    /// ```
    /// let result: i32 = ctx.eval(Source::new("1 + 2")).unwrap();
    /// assert_eq!(result, 3);
    /// ```
    pub fn eval<T>(&self, source: Source) -> JSResult<T>
    where
        C::Value: JSObjectOps,
        T: FromJSValue<C::Value>,
    {
        let result = match source.kind() {
            SourceKind::ByteCode(code) => self.rc.inner.run_bytecode(code),
            SourceKind::JavaScript(code) => self.rc.inner.eval(Source::from_bytes(code.clone())),
        };
        result.try_convert::<T>()
    }

    /// Get the global object of the JavaScript context
    ///
    /// # Returns
    /// A JSObject representing the global object
    ///
    /// # Examples
    /// ```rust
    /// let global = ctx.global();
    /// global.set("myVar", 42);
    /// let result: i32 = global.get("myVar").unwrap();
    /// assert_eq!(result, 42);
    /// ```
    pub fn global(&self) -> JSObject<C::Value>
    where
        C::Value: JSTypeOf,
    {
        let raw = self.rc.inner.global();
        JSObject::from_js_value(self, raw).unwrap()
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
    pub fn register_class<JC>(&self) -> JSResult<()>
    where
        JC: JSClass<C::Value>,
        C::Value: JSObjectOps,
    {
        let registry = self
            .get_class_registry()
            .ok_or(RustyJSError::Error("No Class registry!".to_string()))?;

        if registry.borrow().contains_key(&TypeId::of::<JC>()) {
            return Ok(());
        }

        let value = self.rc.inner.register_class::<JC>();

        let obj = self.global();
        let constructor = JSValue::from_raw(self, value.clone());
        JC::class_setup(&ClassSetup::new(constructor.clone().into(), self))?;
        obj.set(JC::NAME, constructor)?;

        registry.borrow_mut().insert(TypeId::of::<JC>(), value);

        Ok(())
    }

    /// Get class registry from context
    pub(crate) fn get_class_registry(&self) -> Option<&RefCell<HashMap<TypeId, C::Value>>> {
        let data = self.get_opaque();
        if data.is_null() {
            None
        } else {
            unsafe { Some(&(*data).registry) }
        }
    }

    pub fn runtime(&self) -> &JSRuntime<C::Runtime> {
        &self.rc.runtime
    }

    /// Compile JavaScript source code to bytecode
    ///
    /// # Arguments
    /// * `code` - The JavaScript source code to compile. Accepts:
    ///   - &str: JavaScript source code as string
    ///   - &[u8]: JavaScript source code as bytes
    ///   - String: Owned JavaScript source code
    ///   - Vec<u8>: Owned JavaScript source code as bytes
    ///
    /// # Returns
    /// * `Ok(Source)` - Compiled bytecode wrapped in a Source, ready to be evaluated
    /// * `Err(RustyJSError)` - If compilation fails
    ///
    /// # Example
    /// ```rust
    /// // From string literal
    /// let bytecode = ctx.compile_to_bytecode("function add(a, b) { return a + b; }")?;
    /// let result: i32 = ctx.eval(bytecode)?;
    ///
    /// // From bytes
    /// let bytecode = ctx.compile_to_bytecode(b"let x = 1;")?;
    /// ```
    pub fn compile_to_bytecode<T: AsRef<[u8]>>(&self, code: T) -> JSResult<Source> {
        self.rc
            .inner
            .compile_to_bytecode(Source::from_bytes(code.as_ref()))
            .map(Source::from_bytecode)
    }

    /// Evaluate JavaScript code and handle both Promise and immediate results
    ///
    /// This function evaluates the provided JavaScript source code and:
    /// 1. If the result is a Promise, waits for it to resolve and returns the resolved value
    /// 2. If the result is not a Promise, returns it immediately
    ///
    /// # Arguments
    /// * `source` - The JavaScript source code to evaluate
    ///
    /// # Returns
    /// * `Ok(T)` - The result of the evaluation or resolved Promise value
    /// * `Err(RustyJSError)` - If evaluation fails, throws an exception, or Promise rejects
    pub async fn eval_async<T>(&self, source: Source) -> JSResult<T>
    where
        C::Value: JSTypeOf + JSObjectOps + 'static,
        T: FromJSValue<C::Value> + 'static,
    {
        let result = match source.kind() {
            SourceKind::ByteCode(code) => self.rc.inner.run_bytecode(code),
            SourceKind::JavaScript(code) => self.rc.inner.eval(Source::from_bytes(code.clone())),
        };

        match (result.is_promise(), result.is_exception()) {
            (true, _) => {
                let promise = Promise::from_js_value(self, result)?;
                promise.into_future::<T>().await
            }
            (_, true) => {
                let err = JSException::from_js_value(self, result)?;
                Err(RustyJSError::Exception(err.into_error()))
            }
            _ => T::from_js_value(self, result),
        }
    }

    fn get_opaque(&self) -> *mut ContextOpaque<C::Value> {
        let key = self.rc.inner.as_raw();
        Self::_get_opaque(key)
    }

    fn _get_opaque(raw_ctx: &C::RawContext) -> *mut ContextOpaque<C::Value> {
        let key = C::context_id(raw_ctx);
        if let Some(opaque_ptr) = CTX_OPAQUE.read().unwrap().get(&key) {
            *opaque_ptr as *mut ContextOpaque<C::Value>
        } else {
            std::ptr::null_mut()
        }
    }
}

/// Container to hold the context-specific data for a JSContext.
///
/// # Fields
/// - `registry`: A pointer to a RefCell containing a HashMap that maps TypeId to type that implements JSValueImpl
/// - `ctx_inner`: Weak reference to the JSContextInner used to build JSContext from callback case
struct ContextOpaque<V: JSValueImpl> {
    registry: RefCell<HashMap<TypeId, V>>,
    ctx_inner: Weak<JSContextInner<V::Context>>,
}

impl<V: JSValueImpl> ContextOpaque<V> {
    fn new(ctx_inner: Weak<JSContextInner<V::Context>>) -> Box<Self> {
        Box::new(Self {
            registry: RefCell::new(HashMap::new()),
            ctx_inner,
        })
    }
}

// Global HashMap to store ContextOpaque<V>
// Like JavaScriptCore engine, it does not provide API to save opaque on JS Context,
// that's why we introduce general solution CTX_OPAQUE
static CTX_OPAQUE: LazyLock<RwLock<HashMap<usize, usize>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

impl<C: JSContextImpl> Drop for JSContext<C> {
    fn drop(&mut self) {
        if Rc::strong_count(&self.rc) == 1 {
            let data = self.get_opaque();
            if !data.is_null() {
                unsafe {
                    //println!("cleanup context and resources");

                    // cleanup class registry
                    let registry = &(*data).registry;
                    registry.borrow_mut().clear();

                    // cleanup ContextOpaque
                    let _ = Box::from_raw(data);
                }
            }
        }
    }
}

impl<C: JSContextImpl> Clone for JSContext<C> {
    fn clone(&self) -> Self {
        Self {
            rc: Rc::clone(&self.rc),
        }
    }
}

impl<C: JSContextImpl> std::fmt::Debug for JSContext<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "JSContext {{ address: {:p}, ref_count: {} }}",
            self as *const _,
            Rc::strong_count(&self.rc)
        )
    }
}
