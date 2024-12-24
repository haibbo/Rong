use crate::{
    FromJSValue, JSContextImpl, JSExceptionHandler, JSObject, JSObjectOps, JSValue, JSValueImpl,
    RustFunc,
};
use std::any::TypeId;
use std::cell::{Ref, RefCell, RefMut};

/// JSClass trait for rust type that supports TypeId
/// `TypeId` is currently only available for types which ascribe to `'static`,
pub trait JSClass<V: JSValueImpl>: Sized + 'static {
    // the name of class constructor
    const NAME: &'static str;

    fn data_constructor() -> RustFunc<V>;
}

pub trait JSClassExt<V: JSValueImpl>: JSClass<V> {
    fn constructor(ctx: &V::Context, this: V, args: Vec<V>) -> V
    where
        V::Context: JSExceptionHandler<Value = V>,
    {
        Self::data_constructor().call(ctx, this, args).unwrap()
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
        func.call(ctx, this, args).unwrap()
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
    pub fn free<JC: JSClass<V>>(instance: V) {
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
