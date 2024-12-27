use crate::function::ParamsAccessor;
use crate::{
    FromJSValue, FromParams, IntoJSCallable, JSContext, JSContextImpl, JSExceptionHandler, JSFunc,
    JSObject, JSObjectOps, JSValue, JSValueImpl, PropertyDescriptor, PropertyKey, RustFunc,
};

use std::any::TypeId;
use std::cell::{Ref, RefCell, RefMut};

/// JSClass trait for rust type that supports TypeId
/// `TypeId` is currently only available for types which ascribe to `'static`,
pub trait JSClass<V: JSValueImpl>: Sized + 'static {
    // the name of class constructor
    const NAME: &'static str;

    fn data_constructor() -> RustFunc<V>;
    fn class_setup(class: &ClassSetup<V>);
}

pub trait JSClassExt<V: JSValueImpl>: JSClass<V> {
    fn constructor(ctx: &V::Context, this: V, args: Vec<V>) -> V
    where
        V::Context: JSExceptionHandler<Value = V>,
        V: JSObjectOps,
    {
        let proto = Class::from((ctx.clone(), this.clone())).get_prototype();

        let mut accessor = ParamsAccessor::new(ctx, this, args);
        let instance = Self::data_constructor().call(&mut accessor).unwrap();

        let instance = JSObject::from_js_value(ctx, instance).unwrap();
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
        V::Context: JSExceptionHandler<Value = V>,
    {
        let obj = JSObject::from_js_value(ctx, function).unwrap();
        let func = obj.borrow::<RustFunc<_>>().unwrap();

        let mut accessor = ParamsAccessor::new(ctx, this, args);
        func.call(&mut accessor).unwrap()
    }
}

// Blanket implementation
impl<T, V: JSValueImpl> JSClassExt<V> for T where T: JSClass<V> {}

pub struct Class<V: JSValueImpl>(pub(crate) JSObject<V>);

/// caller should make sure V is class constructor
impl<V: JSValueImpl> From<(V::Context, V)> for Class<V> {
    fn from(parts: (V::Context, V)) -> Self {
        let jsvalue: JSValue<V> = parts.into();
        Self(jsvalue.into())
    }
}

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
        let constructor = context
            .get_class_registry_mut()
            .and_then(|registry| registry.get(&TypeId::of::<JC>()))
            .cloned()?;
        Some(Self(JSObject::from_js_value(context, constructor).unwrap()))
    }

    pub fn get_prototype(&self) -> JSObject<V> {
        let obj: JSObject<V> = self.0.get("prototype").unwrap();
        obj
    }
}

impl<V> JSObject<V>
where
    V: JSValueImpl + JSObjectOps,
{
    /// Borrow the underlying data from an instance
    pub fn borrow<T>(&self) -> Option<Ref<'_, T>> {
        let ptr = self.as_inner().get_opaque::<RefCell<T>>();
        if ptr.is_null() {
            None
        } else {
            Some(unsafe { &*ptr }.borrow())
        }
    }

    /// Mutably borrow the underlying data from an instance
    pub fn borrow_mut<T>(&self) -> Option<RefMut<'_, T>> {
        let ptr = self.as_inner().get_opaque::<RefCell<T>>();
        if ptr.is_null() {
            None
        } else {
            // SAFETY: ptr was created by Box::into_raw in instance()
            Some(unsafe { &*ptr }.borrow_mut())
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
        let prototype = constructor.get_prototype();
        Self {
            constructor: constructor.0,
            prototype,
            context,
        }
    }

    pub fn method<F, P>(&self, name: &str, f: F)
    where
        F: IntoJSCallable<V, P> + 'static,
        P: FromParams<V>,
        V: JSObjectOps + 'static,
        V::Context: JSExceptionHandler,
    {
        let func = self.context.register_function(f);
        println!("name is {}", name);
        self.prototype.set(name, func.name(name));
    }

    pub fn static_method<F, P>(&self, name: &str, f: F)
    where
        F: IntoJSCallable<V, P> + 'static,
        P: FromParams<V>,
        V: JSObjectOps + 'static,
        V::Context: JSExceptionHandler,
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

    pub fn new_func<F, P>(&self, f: F) -> JSFunc<V>
    where
        F: IntoJSCallable<V, P> + 'static,
        P: FromParams<V>,
        V: JSObjectOps + 'static,
        V::Context: JSExceptionHandler,
    {
        self.context.register_function(f)
    }
}
