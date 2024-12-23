use crate::{FromJSValue, JSContextImpl, JSObject, JSObjectOps, JSValueImpl, RustFunc};
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
    fn constructor(context: &V::Context, args: &[V]) -> V {
        Self::data_constructor().call(context, args).unwrap()
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

    /// Get class constructor by type
    pub fn get<JC: JSClass<V>>(context: &V::Context) -> Option<Self> {
        let constructor = context
            .get_class_registry_mut()
            .and_then(|registry| registry.get(&TypeId::of::<JC>()))
            .cloned()?;
        Some(Self(JSObject::from_js_value(context, constructor).unwrap()))
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
}
