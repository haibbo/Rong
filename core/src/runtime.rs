use crate::function::RustFunc;
use crate::scheduler::Scheduler;
use crate::{JSContext, JSContextImpl, JSObjectOps, JSResult, JSValueImpl};
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::rc::Rc;
use tokio::sync::Notify;

pub trait JSRuntimeImpl {
    /// The raw runtime handle type associated with this runtime.
    type RawRuntime;

    /// The JavaScript context type associated with this runtime
    type Context: JSContextImpl;

    /// Creates JavaScript runtime.
    fn new() -> Self;

    /// Converts the runtime to its raw underlying representation.
    ///
    /// # Key Notes
    /// - This provides access to the low-level, engine-specific runtime handle.
    /// - Useful for advanced use cases requiring direct interaction with the engine.
    /// - The returned value is a copy of the raw runtime handle.
    fn to_raw(&self) -> Self::RawRuntime;

    /// Runs all pending jobs in the JavaScript runtime.
    /// This includes executing any queued promise callbacks, microtasks, and other pending operations.
    fn run_pending_jobs(&self);

    /// Runs garbage collection on the JavaScript runtime.
    ///
    /// # Key Notes
    /// - This method triggers a garbage collection cycle to reclaim unused memory.
    /// - The exact behavior depends on the underlying JavaScript engine implementation.
    /// - Use this judiciously as it may impact performance.
    fn run_gc(&self);
}

pub struct JSRuntime<R: JSRuntimeImpl> {
    pub(crate) inner: Rc<R>,
    scheduler: Rc<Scheduler<R>>,
    services: ServiceContainer,
    pub(crate) engine: &'static str,
}

impl<R: JSRuntimeImpl> Clone for JSRuntime<R> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            scheduler: self.scheduler.clone(),
            services: self.services.clone(),
            engine: self.engine,
        }
    }
}

impl<R: JSRuntimeImpl + 'static> JSRuntime<R> {
    pub fn block_on<F, T>(&self, future: F) -> JSResult<T>
    where
        F: Future<Output = JSResult<T>> + 'static,
        T: 'static,
    {
        self.scheduler.block_on(future)
    }

    pub(crate) fn scheduler(&self) -> &Rc<Scheduler<R>> {
        &self.scheduler
    }

    pub fn get_shutdown_signal(&self) -> Rc<Notify> {
        self.scheduler.get_shutdown_signal()
    }

    /// # Warning
    /// testing purposes only and don't use it in production code.
    #[doc(hidden)]
    pub fn run_pending_jobs(&self) {
        self.inner.run_pending_jobs();
    }

    /// Runs garbage collection on the JavaScript runtime.
    ///
    /// # Key Notes
    /// - This method triggers a garbage collection cycle to reclaim unused memory.
    /// - The exact behavior depends on the underlying JavaScript engine implementation.
    /// - Use this judiciously as it may impact performance.
    pub fn run_gc(&self) {
        self.inner.run_gc();
    }

    /// Get or initialize a runtime service
    /// If the service is not registered, it will be initialized with default values
    pub fn get_or_init_service<T: JSRuntimeService + Default>(&self) -> &T {
        if let Some(service) = self.services.get::<T>() {
            service
        } else {
            let service = T::default();
            self.services.register(service);
            self.services
                .get::<T>()
                .expect("Service should be registered")
        }
    }
}

impl<C: JSContextImpl> JSContext<C> {
    /// used to create object instance as function
    pub(crate) fn register_builtin_class(&self) -> JSResult<()>
    where
        C::Value: JSObjectOps + 'static,
    {
        self.register_class::<RustFunc<C::Value>>()
    }
}

pub trait JSEngine: Sized {
    type Value: JSValueImpl<Context = Self::Context>;
    type Context: JSContextImpl<Value = Self::Value, Runtime = Self::Runtime>;
    type Runtime: JSRuntimeImpl<Context = Self::Context> + 'static;

    /// JS engine name
    fn name() -> &'static str;

    /// JS engine version
    fn version() -> String;

    /// Creates a new JavaScript runtime instance.
    ///
    /// # Key Notes
    /// - One thread have only one runtime instance.
    /// - Each runtime has its own scheduler for managing asynchronous tasks.
    /// - This ensures proper isolation and thread-safety in JavaScript execution.
    ///
    /// # Returns
    /// A new `JSRuntime` instance with its associated scheduler.
    fn runtime() -> JSRuntime<Self::Runtime> {
        let runtime = Rc::new(Self::Runtime::new());
        let scheduler = Scheduler::new(runtime.clone());
        JSRuntime {
            inner: runtime,
            scheduler,
            services: ServiceContainer::new(),
            engine: Self::name(),
        }
    }

    /// Creates a new JavaScript context instance associated with the given runtime.
    ///
    /// # Key Notes
    /// - Each context is tied to a specific runtime instance.
    /// - Automatically registers the Rust function class for interop capabilities.
    /// - Contexts are isolated execution environments within the same runtime.
    ///
    /// # Example
    /// ```rust
    /// let runtime = JSEngine::runtime();
    /// let context = JSEngine::context(&runtime);
    /// ```
    fn context(rt: &JSRuntime<Self::Runtime>) -> JSContext<Self::Context>
    where
        Self::Value: JSObjectOps + 'static,
    {
        let ctx = JSContext::<Self::Context>::new(rt);
        ctx.register_builtin_class()
            .expect("Failed to register builtin class");

        ctx.global()
            .set("Rong", ctx.rong())
            .expect("Failed to add Rong object");

        ctx
    }
}

/// A trait for runtime services that can be attached to JSRuntime
pub trait JSRuntimeService: 'static {
    /// Called when the service is being initialized
    fn on_init(&self) {}

    /// Called when the service is being shutdown
    fn on_shutdown(&self) {}
}

/// A container for runtime services with proper lifecycle management
#[derive(Clone)]
struct ServiceContainer {
    services: Rc<RefCell<HashMap<TypeId, Box<dyn JSRuntimeService>>>>,
}

impl ServiceContainer {
    fn new() -> Self {
        Self {
            services: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    fn register<T: JSRuntimeService>(&self, extension: T) {
        let mut services = self.services.borrow_mut();
        extension.on_init();
        services.insert(TypeId::of::<T>(), Box::new(extension));
    }

    fn get<T: JSRuntimeService>(&self) -> Option<&T> {
        // SAFETY: This is safe because:
        // 1. We only insert services through register<T>
        // 2. TypeId is unique for each type
        // 3. The extension is never removed until container is dropped
        // 4. The RefCell ensures we don't have multiple mutable borrows
        unsafe {
            let services = self.services.borrow();
            services
                .get(&TypeId::of::<T>())
                .map(|ext| &*(ext.as_ref() as *const dyn JSRuntimeService as *const T))
        }
    }

    fn shutdown(&self) {
        let mut services = self.services.borrow_mut();
        for (_, ext) in services.drain() {
            ext.on_shutdown();
        }
    }
}

impl Drop for ServiceContainer {
    fn drop(&mut self) {
        // Only shutdown if we're the last reference
        if Rc::strong_count(&self.services) == 1 {
            self.shutdown();
        }
    }
}
