use crate::function::{Constructor, FromParams, IntoJSCallable, ParamsAccessor, RustFunc};
use crate::{
    FromJSValue, HostError, JSArrayOps, JSContext, JSContextImpl, JSErrorFactory,
    JSExceptionThrower, JSFunc, JSObject, JSObjectOps, JSResult, JSValue, JSValueImpl,
    PropertyDescriptor, PropertyKey, RongJSError,
};

use std::any::TypeId;
use std::cell::{Ref, RefCell, RefMut};
use std::ops::Deref;

/// JSClass trait for rust type that supports TypeId
/// `TypeId` is currently only available for types which ascribe to `'static`,
pub trait JSClass<V: JSValueImpl>: Sized + 'static {
    // the name of class constructor
    const NAME: &'static str;

    /// Returns the data constructor function for this class
    fn data_constructor() -> Constructor<V>;

    /// Returns the implicit constructor function for this class
    ///
    /// This is the function called when the class is invoked directly without
    /// 'new' (e.g. `MyClass()`).
    ///
    /// Note: This is distinct from `data_constructor()`, which provides the internal
    /// constructor logic.
    fn call_without_new() -> Constructor<V> {
        Constructor::new(|| ())
    }

    /// Configures the class prototype and constructor with methods and properties
    fn class_setup(class: &ClassSetup<V>) -> JSResult<()>;

    /// Marks JavaScript values held by this object to prevent garbage collection.
    ///
    /// Used primarily by the QuickJS engine. Some engines like JavaScriptCore
    /// handle reference tracking automatically.
    ///
    /// IMPORTANT: Do NOT use clone() inside this method as it may break reference
    /// counting in the garbage collector.
    ///
    /// Default implementation does nothing. Override this when you need to prevent
    /// JS objects referenced by Rust from being garbage collected.
    fn gc_mark_with<F>(&self, _mark_fn: F)
    where
        F: FnMut(&JSValue<V>),
    {
        // Default implementation does nothing
    }
}

pub trait JSClassExt<V: JSValueImpl>: JSClass<V> {
    fn constructor(ctx: &V::Context, this: V, args: Vec<V>) -> V
    where
        V::Context: JSErrorFactory + JSExceptionThrower,
        V: JSObjectOps + JSArrayOps,
    {
        let ctx = &JSContext::from_borrowed_raw_ptr(ctx.as_raw());
        let mut accessor = ParamsAccessor::new(ctx, this.clone(), args);

        if this.is_undefined() {
            match Self::call_without_new().0.call(&mut accessor) {
                Ok(v) => return v,
                Err(e) => return e.throw_js_exception(ctx),
            }
        }

        let instance = match Self::data_constructor().0.call(&mut accessor) {
            Ok(v) => {
                if v.is_exception() {
                    return v;
                }
                v
            }
            Err(e) => return e.throw_js_exception(ctx),
        };

        let instance = match JSObject::from_js_value(ctx, JSValue::from_raw(ctx, instance)) {
            Ok(obj) => obj,
            Err(e) => return e.throw_js_exception(ctx),
        };

        // why not using constructor in class registry?
        // because for extended class, it's NEW Constructor
        let proto = match JSObject::from_js_value(ctx, JSValue::from_raw(ctx, this)).and_then(
            |constructor| {
                // println!(
                //     "Constructor Name is {}",
                //     constructor.get::<_, String>("name").unwrap()
                // );
                constructor.get("prototype")
            },
        ) {
            Ok(proto) => proto,
            Err(e) => return e.throw_js_exception(ctx),
        };

        instance.prototype(proto);
        instance.into_value()
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
        V: JSObjectOps + JSArrayOps + 'static,
        V::Context: JSErrorFactory + JSExceptionThrower,
    {
        let ctx = &JSContext::from_borrowed_raw_ptr(ctx.as_raw());
        let mut accessor = ParamsAccessor::new(ctx, this, args);

        let obj = match JSObject::from_js_value(ctx, JSValue::from_raw(ctx, function)) {
            Ok(obj) => obj,
            Err(e) => return e.throw_js_exception(ctx),
        };

        let mut func = match obj.borrow_mut::<RustFunc<V>>() {
            Ok(f) => f,
            Err(_) => return RongJSError::NotJSFunc().throw_js_exception(ctx),
        };

        // ignore checking whether v is exception or error, sicne no one use it again here
        match func.call(&mut accessor) {
            Ok(v) => v,
            Err(e) => e.throw_js_exception(ctx),
        }
    }
}

// Blanket implementation
impl<T, V: JSValueImpl> JSClassExt<V> for T where T: JSClass<V> {}

/// Represents a JavaScript class constructor
///
/// This struct encapsulates a JavaScript object that serves as a class constructor.
/// It is used to create class instances and manage class lifecycle.
pub struct Class<V: JSValueImpl>(pub(crate) JSObject<V>);

impl<V> Deref for Class<V>
where
    V: JSValueImpl,
{
    type Target = JSObject<V>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> Class<V>
where
    V: JSValueImpl + JSObjectOps,
{
    /// Create a new instance of the class
    pub fn instance<JC: JSClass<V>>(self, value: JC) -> JSObject<V> {
        let context = self.0.get_ctx();
        let ptr = Box::into_raw(Box::new(RefCell::new(value)));

        let instance = V::make_instance(context.as_ref(), self.0.clone().into_value(), ptr as _);

        // instance object inherits Constructor's prototype
        let _ = self
            .0
            .get::<_, JSObject<V>>("prototype")
            .map(|proto| instance.set_prototype(proto.into_value()));
        JSObject::from_raw(&context, instance)
    }

    /// Check if the object is an instance of the specified class
    pub fn instance_of<JC: JSClass<V>>(object: &JSObject<V>) -> bool {
        let context = object.get_ctx();
        if let Ok(class) = Class::<V>::get::<JC>(&context) {
            object.as_value().instance_of(class.0.into_value())
        } else {
            false
        }
    }

    /// Free resources of a class instance
    pub(crate) fn free<JC: JSClass<V>>(instance: V) {
        let value = instance.clone();
        let ptr = value.get_opaque() as *mut RefCell<JC>;
        if !ptr.is_null() {
            // SAFETY: ptr was created by Box::into_raw in instance()
            unsafe {
                let _ = Box::from_raw(ptr);
            };
        }
    }

    /// Get class constructor by type
    pub fn get<JC: JSClass<V>>(context: &JSContext<V::Context>) -> JSResult<Self> {
        let constructor = context
            .get_class_registry()
            .and_then(|registry| registry.borrow().get(&TypeId::of::<JC>()).cloned())
            .ok_or_else(|| {
                HostError::new(
                    crate::error::E_INVALID_STATE,
                    format!("JS Class {} is not registered", std::any::type_name::<JC>()),
                )
            })?;

        Ok(Self(JSObject::from_raw(context, constructor)))
    }

    pub fn prototype<JC: JSClass<V>>(context: &JSContext<V::Context>) -> JSResult<JSObject<V>> {
        let class = Class::get::<JC>(context)?;
        class.0.get::<_, JSObject<V>>("prototype")
    }

    /// Construct a Class constructor from a JSObject if it is an instance of the specified class
    ///
    /// # Arguments
    /// * `obj` - The JSObject to check and convert
    ///
    /// # Returns
    /// - `Some(Class)` if the object is an instance of the specified class
    /// - `None` if the object is not an instance of the specified class
    pub fn from_object<JC: JSClass<V>>(obj: &JSObject<V>) -> Option<Self> {
        if Self::instance_of::<JC>(obj) {
            Some(Self(obj.clone()))
        } else {
            None
        }
    }
}

impl<V> JSObject<V>
where
    V: JSValueImpl + JSObjectOps,
{
    /// Borrow the underlying data from an instance
    pub fn borrow<T>(&self) -> JSResult<Ref<'_, T>>
    where
        T: JSClass<V>,
    {
        if !Class::instance_of::<T>(self) {
            return Err(HostError::new(
                crate::error::E_TYPE,
                format!("Not instance of {}", std::any::type_name::<T>()),
            )
            .with_name("TypeError")
            .into());
        }

        let ptr = self.as_value().get_opaque() as *mut RefCell<T>;
        if ptr.is_null() {
            Err(RongJSError::Borrow(std::any::type_name::<T>()))
        } else {
            // SAFETY: ptr was created by Box::into_raw in instance()
            Ok(unsafe { &*ptr }.borrow())
        }
    }

    /// Mutably borrow the underlying data from an instance
    pub fn borrow_mut<T>(&self) -> JSResult<RefMut<'_, T>>
    where
        T: JSClass<V>,
    {
        if !Class::instance_of::<T>(self) {
            return Err(HostError::new(
                crate::error::E_TYPE,
                format!("Not instance of {}", std::any::type_name::<T>()),
            )
            .with_name("TypeError")
            .into());
        }

        let ptr = self.as_value().get_opaque() as *mut RefCell<T>;
        if ptr.is_null() {
            Err(RongJSError::Borrow(std::any::type_name::<T>()))
        } else {
            // SAFETY: ptr was created by Box::into_raw in instance()
            Ok(unsafe { &*ptr }.borrow_mut())
        }
    }

    pub fn prototype(&self, proto: JSObject<V>) -> bool {
        self.as_value().set_prototype(proto.into_value())
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
            .0
            .get::<_, JSObject<V>>("prototype")
            .expect("Class prototype not found");
        Self {
            constructor: constructor.0,
            prototype,
            context,
        }
    }

    /// Access the underlying JS context
    pub fn context(&self) -> &JSContext<V::Context> {
        self.context
    }

    /// Access the prototype object of this class
    pub fn prototype_object(&self) -> JSObject<V> {
        self.prototype.clone()
    }

    pub fn method<F, P, K: 'static>(&self, name: &str, f: F) -> JSResult<()>
    where
        F: IntoJSCallable<V, P, K> + 'static,
        P: FromParams<V>,
        V: JSObjectOps + 'static,
    {
        let func = JSFunc::new(self.context, f)?;
        let func = func.name(name)?;
        self.prototype.set(name, func)?;
        Ok(())
    }

    pub fn static_method<F, P, K: 'static>(&self, name: &str, f: F) -> JSResult<()>
    where
        F: IntoJSCallable<V, P, K> + 'static,
        P: FromParams<V>,
        V: JSObjectOps + 'static,
    {
        let func = JSFunc::new(self.context, f)?;
        let func = func.name(name)?;
        self.constructor.set(name, func)?;
        Ok(())
    }

    pub fn property<Fun, Key>(&self, k: Key, f: Fun) -> JSResult<()>
    where
        Fun: Fn(PropertyDescriptor<V>) -> JSResult<PropertyDescriptor<V>>,
        Key: for<'b> Into<PropertyKey<'b, V>>,
    {
        f(PropertyDescriptor::builder())?.apply_to(&self.prototype, k);
        Ok(())
    }

    pub fn static_property<Fun, Key>(&self, k: Key, f: Fun) -> JSResult<()>
    where
        Fun: Fn(PropertyDescriptor<V>) -> JSResult<PropertyDescriptor<V>>,
        Key: for<'b> Into<PropertyKey<'b, V>>,
    {
        f(PropertyDescriptor::builder())?.apply_to(&self.constructor, k);
        Ok(())
    }

    pub fn new_func<F, P, K: 'static>(&self, f: F) -> JSResult<JSFunc<V>>
    where
        F: IntoJSCallable<V, P, K> + 'static,
        P: FromParams<V>,
        V: JSObjectOps + 'static,
    {
        JSFunc::new(self.context, f)
    }
}
