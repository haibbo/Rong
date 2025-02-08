use crate::{
    FromJSValue, JSContext, JSResult, JSTypeOf, JSValue, JSValueConversion, JSValueImpl,
    RustyJSError,
};
use std::fmt;
use std::ops::Deref;

mod property;
pub use property::{PropertyAttributes, PropertyDescriptor, PropertyKey};

use super::IntoJSValue;

pub struct JSObject<V: JSValueImpl>(JSValue<V>);

impl<V: JSValueImpl> Clone for JSObject<V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<V> From<JSValue<V>> for JSObject<V>
where
    V: JSValueImpl,
{
    fn from(v: JSValue<V>) -> Self {
        JSObject(v)
    }
}

impl<V> FromJSValue<V> for JSObject<V>
where
    V: JSTypeOf,
{
    fn from_js_value(ctx: &JSContext<V::Context>, value: V) -> JSResult<Self> {
        if value.is_object() {
            Ok(JSValue::from_raw(ctx, value).into())
        } else {
            Err(RustyJSError::NotObject)
        }
    }
}

impl<V: JSValueImpl> Deref for JSObject<V> {
    type Target = JSValue<V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> IntoJSValue<V> for JSObject<V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, _ctx: &JSContext<V::Context>) -> V {
        self.0.into_value()
    }
}

pub trait JSObjectOps: JSValueConversion + JSTypeOf {
    /// Creates a new empty object in the given context.
    /// Returns EXCEPTION if creation fails.
    fn new_object(ctx: &Self::Context) -> Self;

    /// Creates a new instance using the given constructor and private data.
    /// Returns EXCEPTION if instantiation fails.
    ///
    /// # Arguments
    /// * `ctx` - The JavaScript context
    /// * `constructor` - The constructor function (JS Class)
    /// * `data` - Pointer to private data to store in the object
    fn make_instance(ctx: &Self::Context, constructor: Self, data: *mut ()) -> Self;

    /// Checks if this object is an instance of the given constructor.
    fn instance_of(&self, constructor: Self) -> bool;

    /// Gets the private data stored in the object.
    /// Returns a raw pointer to the opaque data.
    fn get_opaque(&self) -> *mut ();

    /// Deletes a property from the object.
    /// Returns true if the property was successfully deleted.
    fn del_property(&self, key: Self) -> bool;

    /// Checks if the object has the specified property.
    fn has_property(&self, key: Self) -> bool;

    /// Sets a property on the object with the given value.
    /// Returns true if the property was successfully set.
    fn set_property(&self, key: Self, value: Self) -> bool;

    /// Sets the prototype of the object.
    /// Returns true if the prototype was successfully set.
    fn set_prototype(&self, prototype: Self) -> bool;

    /// Defines a property with the given attributes and optional getter/setter.
    /// Returns true if the property was successfully defined.
    ///
    /// # Arguments
    /// * `key` - The property key
    /// * `value` - The property value
    /// * `getter` - Optional getter function
    /// * `setter` - Optional setter function
    /// * `attributes` - Property attributes (writable, enumerable, configurable)
    fn define_property(
        &self,
        key: Self,
        value: Self,
        getter: Self,
        setter: Self,
        attributes: PropertyAttributes,
    ) -> bool;

    /// Gets the value of a property.
    /// Returns Some(value) if the property exists, None otherwise.
    /// Returns EXCEPTION if the operation fails.
    fn get_property(&self, key: Self) -> Option<Self>;
}

impl<V> JSObject<V>
where
    V: JSObjectOps,
{
    /// new a general object
    pub fn new(ctx: &JSContext<V::Context>) -> Self {
        let value = V::new_object(ctx.as_ref());
        JSObject::from_js_value(ctx, value).unwrap()
    }

    pub(crate) fn into_value(self) -> V {
        self.0.into_value()
    }

    pub(crate) fn as_mut_value(&mut self) -> &mut V {
        &mut self.0.inner
    }
}

impl<V> JSObject<V>
where
    V: JSObjectOps,
{
    pub fn set<'a, K, KV>(&'a self, k: K, kv: KV) -> bool
    where
        K: Into<PropertyKey<'a>>,
        KV: IntoJSValue<V>,
    {
        let ctx = &self.get_ctx();
        let key = k.into().into_key(ctx);
        self.as_value().set_property(key, kv.into_js_value(ctx))
    }

    pub fn del<'a, K>(&'a self, k: K) -> bool
    where
        K: Into<PropertyKey<'a>>,
    {
        let key = k.into().into_key(&self.get_ctx());
        self.as_value().del_property(key)
    }

    pub fn has<'a, K>(&self, k: K) -> bool
    where
        K: Into<PropertyKey<'a>>,
    {
        let key = k.into().into_key(&self.get_ctx());
        self.as_value().has_property(key)
    }

    pub fn get<'a, K, T>(&'a self, k: K) -> JSResult<T>
    where
        K: Into<PropertyKey<'a>>,
        T: FromJSValue<V>,
    {
        let ctx = &self.get_ctx();
        let key = k.into().into_key(ctx);
        self.as_value()
            .get_property(key)
            .ok_or(RustyJSError::PropertyNotFound) // check existence firstly
            .and_then(|value| T::from_js_value(ctx, value))
    }
}

// blanket implementing.
impl<V: JSValueImpl> crate::function::JSParameterType for JSObject<V> {}

impl<V> fmt::Display for JSObject<V>
where
    V: JSTypeOf + JSValueConversion,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Delegate to JSValue's Display implementation through Deref
        self.deref().fmt(f)
    }
}

impl<V> fmt::Debug for JSObject<V>
where
    V: JSTypeOf + JSValueConversion,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSObject({})", self)
    }
}
