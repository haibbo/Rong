use crate::function::{Constructor, FromParams, IntoJSCallable, ParamsAccessor, RustFunc};
use crate::{
    FromJSValue, JSContext, JSExceptionHandler, JSFunc, JSObject, JSObjectOps, JSResult,
    JSValueImpl, PropertyDescriptor, PropertyKey, RustyJSError,
};

use std::any::TypeId;
use std::cell::{Ref, RefCell, RefMut};

/// JSClass trait for rust type that supports TypeId
/// `TypeId` is currently only available for types which ascribe to `'static`,
pub trait JSClass<V: JSValueImpl>: Sized + 'static {
    // the name of class constructor
    const NAME: &'static str;

    /// Returns the data constructor function for this class
    fn data_constructor() -> Constructor<V>;

    /// Configures the class prototype and constructor with methods and properties
    fn class_setup(class: &ClassSetup<V>);
}

pub trait JSClassExt<V: JSValueImpl>: JSClass<V> {
    fn constructor(ctx: &V::Context, this: V, args: Vec<V>) -> V
    where
        V::Context: JSExceptionHandler,
        V: JSObjectOps,
    {
        let mut accessor = ParamsAccessor::new(ctx, this.clone(), args);

        let instance = match Self::data_constructor().0.call(&mut accessor) {
            Ok(v) => v,
            Err(e) => return e.throw_js_exception(ctx),
        };

        let instance = match JSObject::from_js_value(ctx, instance) {
            Ok(obj) => obj,
            Err(e) => return e.throw_js_exception(ctx),
        };

        let proto = match JSObject::from_js_value(ctx, this)
            .and_then(|constructor| constructor.get("prototype"))
        {
            Ok(proto) => proto,
            Err(e) => return e.throw_js_exception(ctx),
        };

        instance.prototype(proto);
        instance.into_inner()
    }

    /// Free resources of a class instance by finalizer
    fn free(value: V)
    where
        V: JSObjectOps,
    {
        Class::free::<Self>(value);
    }

    /// call object as function
    fn call(ctx: &V::Context, function: V, this: V, args: Vec<V>) -> V
    where
        V: JSObjectOps,
        V::Context: JSExceptionHandler,
    {
        let mut accessor = ParamsAccessor::new(ctx, this, args);

        let obj = match JSObject::from_js_value(ctx, function) {
            Ok(obj) => obj,
            Err(e) => return e.throw_js_exception(ctx),
        };

        let mut func = match obj.borrow_mut::<RustFunc<V>>() {
            Ok(f) => f,
            Err(_) => return RustyJSError::NotJSFunc.throw_js_exception(ctx),
        };

        match func.call(&mut accessor) {
            Ok(v) => v,
            Err(e) => e.throw_js_exception(ctx),
        }
    }
}

// Blanket implementation
impl<T, V: JSValueImpl> JSClassExt<V> for T where T: JSClass<V> {}

pub struct Class<V: JSValueImpl>(pub(crate) JSObject<V>);

impl<V> Class<V>
where
    V: JSValueImpl + JSObjectOps,
{
    /// Create a new instance of the class
    pub fn instance<JC: JSClass<V>>(self, value: JC) -> V {
        let context = self.0.as_ctx();
        let ptr = Box::into_raw(Box::new(RefCell::new(value)));
        V::make_object(context, self.0.clone().into_inner(), ptr)
    }

    /// Free resources of a class instance
    pub(crate) fn free<JC: JSClass<V>>(instance: V) {
        let value = instance.clone();
        let ptr = value.get_opaque::<RefCell<JC>>();
        if !ptr.is_null() {
            // SAFETY: ptr was created by Box::into_raw in instance()
            unsafe {
                let _ = Box::from_raw(ptr);
            };
        }
    }

    /// Get class constructor by type
    pub fn get<JC: JSClass<V>>(context: &V::Context) -> Option<Self> {
        let ctx = JSContext::from_raw_ptr(context);
        let constructor = ctx
            .get_class_registry()
            .and_then(|registry| registry.borrow().get(&TypeId::of::<JC>()).cloned())?;

        match JSObject::from_js_value(context, constructor) {
            Ok(obj) => Some(Self(obj)),
            Err(_) => None,
        }
    }

    pub fn get_prototype(&self) -> Option<JSObject<V>> {
        self.0.get("prototype").ok()
    }
}

impl<V> JSObject<V>
where
    V: JSValueImpl + JSObjectOps,
{
    /// Borrow the underlying data from an instance
    pub fn borrow<T>(&self) -> JSResult<Ref<'_, T>> {
        let ptr = self.as_inner().get_opaque::<RefCell<T>>();
        if ptr.is_null() {
            Err(RustyJSError::Borrow(std::any::type_name::<T>()))
        } else {
            // SAFETY: ptr was created by Box::into_raw in instance()
            Ok(unsafe { &*ptr }.borrow())
        }
    }

    /// Mutably borrow the underlying data from an instance
    pub fn borrow_mut<T>(&self) -> JSResult<RefMut<'_, T>> {
        let ptr = self.as_inner().get_opaque::<RefCell<T>>();
        if ptr.is_null() {
            Err(RustyJSError::Borrow(std::any::type_name::<T>()))
        } else {
            // SAFETY: ptr was created by Box::into_raw in instance()
            Ok(unsafe { &*ptr }.borrow_mut())
        }
    }

    pub fn prototype(&self, proto: JSObject<V>) -> bool {
        self.as_inner().set_prototype(proto.into_inner())
    }
}

pub struct ClassSetup<'a, V: JSValueImpl> {
    constructor: JSObject<V>,
    prototype: JSObject<V>,
    context: &'a JSContext<V::Context>,
}

impl<'a, V> ClassSetup<'a, V>
where
    V: JSObjectOps,
{
    pub(crate) fn new(constructor: JSObject<V>, context: &'a JSContext<V::Context>) -> Self {
        let constructor = Class(constructor);
        let prototype = constructor
            .get_prototype()
            .expect("Class prototype not found");
        Self {
            constructor: constructor.0,
            prototype,
            context,
        }
    }

    pub fn method<F, P, K: 'static>(&self, name: &str, f: F)
    where
        F: IntoJSCallable<V, P, K> + 'static,
        P: FromParams<V>,
        V: JSObjectOps + 'static,
    {
        let func = self.context.register_function(f);
        self.prototype.set(name, func.name(name));
    }

    pub fn static_method<F, P, K: 'static>(&self, name: &str, f: F)
    where
        F: IntoJSCallable<V, P, K> + 'static,
        P: FromParams<V>,
        V: JSObjectOps + 'static,
    {
        let func = self.context.register_function(f);
        self.constructor.set(name, func.name(name));
    }

    pub fn property<Fun, Key>(&self, k: Key, f: Fun)
    where
        Fun: Fn(PropertyDescriptor<V>) -> PropertyDescriptor<V>,
        Key: for<'b> Into<PropertyKey<'b>>,
    {
        f(PropertyDescriptor::builder()).apply_to(&self.prototype, k);
    }

    pub fn static_property<Fun, Key>(&self, k: Key, f: Fun)
    where
        Fun: Fn(PropertyDescriptor<V>) -> PropertyDescriptor<V>,
        Key: for<'b> Into<PropertyKey<'b>>,
    {
        f(PropertyDescriptor::builder()).apply_to(&self.constructor, k);
    }

    pub fn new_func<F, P, K: 'static>(&self, f: F) -> JSFunc<V>
    where
        F: IntoJSCallable<V, P, K> + 'static,
        P: FromParams<V>,
        V: JSObjectOps + 'static,
    {
        self.context.register_function(f)
    }
}
