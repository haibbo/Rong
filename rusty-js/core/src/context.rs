use crate::{
    source::Source, ClassSetup, FromJSValue, JSClass, JSObject, JSObjectOps, JSRuntimeImpl,
    JSValue, JSValueImpl, RustyJSError,
};
use std::any::TypeId;
use std::collections::HashMap;
use std::ops::Deref;

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

    fn new(runtime: &Self::Runtime) -> Self
    where
        Self: Sized;

    /// FfiContext implements Copy
    fn to_ffi(&self) -> Self::FfiContext;

    /// the implementation need to make sure it has the ownship, like as new method
    /// generally, it should increase referen count of FFI Context
    fn from_ffi(ctx: Self::FfiContext) -> Self;

    /// Set opaque data for the context
    fn set_opaque<T>(&self, data: *mut T);

    /// Get opaque data from the context
    fn get_opaque<T>(&self) -> *mut T;

    fn init_class_registry(&self) {
        let registry: HashMap<TypeId, Self::Value> = HashMap::new();
        let boxed_registry = Box::new(registry);
        self.set_opaque(Box::into_raw(boxed_registry));
    }

    fn get_class_registry(&self) -> Option<&HashMap<TypeId, Self::Value>> {
        let ptr = self.get_opaque::<HashMap<TypeId, Self::Value>>();
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { &*ptr })
        }
    }

    fn get_class_registry_mut(&self) -> Option<&mut HashMap<TypeId, Self::Value>> {
        let ptr = self.get_opaque::<HashMap<TypeId, Self::Value>>();
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { &mut *ptr })
        }
    }
}

pub trait JSFfiContext {
    type FfiContext;
}

pub struct JSContext<C: JSContextImpl> {
    pub(crate) inner: C,
}

impl<C: JSContextImpl> Deref for JSContext<C> {
    type Target = C;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<C: JSContextImpl> Drop for JSContext<C> {
    fn drop(&mut self) {
        // Clear all constructors in the registry
        // HashMap::clear() will call drop on each Value
        if let Some(registry) = self.inner.get_class_registry_mut() {
            registry.clear()
        }

        //The inner field will be cleaned up by its own Drop implementation
    }
}

impl<C: JSContextImpl> From<C> for JSContext<C> {
    fn from(c: C) -> Self {
        if c.get_class_registry().is_none() {
            c.init_class_registry();
        }
        Self { inner: c }
    }
}

pub trait JSCodeRunner: JSContextImpl {
    /// Evaluate JavaScript code
    fn eval(&self, source: Source) -> Self::Value;

    /// Get global object
    fn global_object(&self) -> Self::Value;

    /// Register class for rust type
    fn register_class<JC>(&self) -> Self::Value
    where
        JC: JSClass<Self::Value>;
}

impl<C> JSContext<C>
where
    C: JSCodeRunner,
{
    /// eval javascript
    pub fn eval<T>(&self, source: Source) -> Result<T, RustyJSError>
    where
        C::Value: JSObjectOps,
        T: FromJSValue<C::Value>,
    {
        let raw = self.inner.eval(source);
        let result = JSValue::new(self, raw);

        if let Some(ex) = result.is_exception() {
            Err(RustyJSError::Error(format!("{}", ex)))
        } else {
            T::from_js_value(&self.inner, result.into_inner())
        }
    }

    /// get global object
    pub fn global_object(&self) -> JSObject<C::Value> {
        let raw = self.inner.global_object();
        JSValue::new(self, raw).into()
    }

    pub fn register_class<JC>(&self)
    where
        JC: JSClass<C::Value>,
        C::Value: JSObjectOps,
    {
        let constructor = self.inner.register_class::<JC>();

        if let Some(registry) = self.inner.get_class_registry_mut() {
            registry.insert(TypeId::of::<JC>(), constructor.clone());
        }

        let obj = self.global_object();
        let constructor = JSValue::new(self, constructor);
        JC::class_setup(&ClassSetup::new(constructor.clone().into(), self));
        obj.set(JC::NAME, constructor);
    }

    pub fn get_class_constructor<JC>(&self) -> Option<&C::Value>
    where
        JC: JSClass<C::Value>,
    {
        self.inner
            .get_class_registry()
            .and_then(|registry| registry.get(&TypeId::of::<JC>()))
    }
}
