use crate::{
    FromJSValue, JSContext, JSFunc, JSResult, JSTypeOf, JSValue, JSValueConversion, JSValueImpl,
    JsonToJsValue, RongJSError,
};
use std::fmt;
use std::ops::Deref;

mod property;
pub use property::{PropertyAttributes, PropertyDescriptor, PropertyKey};

use super::IntoJSValue;

#[derive(Hash, PartialEq, Eq)]
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
            Err(RongJSError::NotObject)
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

    /// Gets the names of all enumerable properties of the object.
    /// Returns None if the operation fails.
    fn get_own_property_names(&self) -> Option<Vec<Self>>;
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

    /// Creates a JSObject from a raw JSValue and context
    pub fn from_raw(ctx: &JSContext<V::Context>, value: V) -> Self {
        JSValue::from_raw(ctx, value).into()
    }

    /// Creates a JSObject from a JSON string
    ///
    /// # Example
    /// ```
    /// let json = r#"{"name":"John","age":30}"#;
    /// let obj = JSObject::from_json_string(&ctx, json)?;
    /// ```
    pub fn from_json_string(ctx: &JSContext<V::Context>, json: &str) -> JSResult<Self> {
        let v = json.json_to_jsvalue(ctx)?;
        Ok(JSObject(v))
    }

    /// Convert JSObject to JSON string using JavaScript's JSON.stringify
    pub fn json_stringify(self) -> JSResult<String> {
        let ctx = self.get_ctx();

        // Get the global JSON object
        let json = ctx.global().get::<_, JSObject<V>>("JSON")?;

        // Get the stringify function
        let stringify = json.get::<_, JSFunc<V>>("stringify")?;

        // Call stringify with this object
        stringify.call::<_, String>(None, (self,))
    }

    pub fn set<'a, K, KV>(&'a self, k: K, kv: KV) -> JSResult<&'a Self>
    where
        K: Into<PropertyKey<'a, V>>,
        KV: IntoJSValue<V>,
    {
        let ctx = &self.get_ctx();
        let key = k.into().into_value(ctx);
        // TODO: handler other err
        self.as_value().set_property(key, kv.into_js_value(ctx));
        Ok(self)
    }

    pub fn del<'a, K>(&'a self, k: K) -> bool
    where
        K: Into<PropertyKey<'a, V>>,
    {
        let key = k.into().into_value(&self.get_ctx());
        self.as_value().del_property(key)
    }

    pub fn has<'a, K>(&self, k: K) -> bool
    where
        K: Into<PropertyKey<'a, V>>,
    {
        let key = k.into().into_value(&self.get_ctx());
        self.as_value().has_property(key)
    }

    pub fn get<'a, K, T>(&'a self, k: K) -> JSResult<T>
    where
        K: Into<PropertyKey<'a, V>>,
        T: FromJSValue<V>,
    {
        let ctx = &self.get_ctx();
        let key = k.into();
        let kv = key.clone().into_value(ctx);
        self.as_value()
            .get_property(kv)
            .ok_or(RongJSError::PropertyNotFound(key.to_string())) // check existence firstly
            .and_then(|value| T::from_js_value(ctx, value))
    }
}

impl<V: JSValueImpl> JSObject<V> {
    /// Converts the JSObject into its underlying/raw JSValue implementation
    pub fn into_value(self) -> V {
        self.0.into_value()
    }

    /// Converts the JSObject into a JSValue
    pub fn into_jsvalue(self) -> JSValue<V> {
        self.0
    }

    pub fn as_jsvalue(&self) -> &JSValue<V> {
        &self.0
    }

    /// Returns a mutable reference to the underlying/raw JSValue
    pub fn as_mut_value(&mut self) -> &mut V {
        &mut self.0.inner
    }
}

pub struct Entry<V: JSValueImpl> {
    key: JSValue<V>,
    value: JSValue<V>,
}

impl<V: JSValueImpl> Entry<V> {
    pub fn key(&self) -> &JSValue<V> {
        &self.key
    }

    pub fn value(&self) -> &JSValue<V> {
        &self.value
    }

    pub fn into_tuple(self) -> (JSValue<V>, JSValue<V>) {
        (self.key, self.value)
    }

    pub fn try_into<K, T>(self) -> JSResult<(K, T)>
    where
        K: FromJSValue<V>,
        T: FromJSValue<V>,
    {
        let ctx = self.key.get_ctx();
        Ok((
            K::from_js_value(&ctx, self.key.into_value())?,
            T::from_js_value(&ctx, self.value.into_value())?,
        ))
    }
}

impl<V> JSObject<V>
where
    V: JSObjectOps,
{
    /// Returns an iterator over the object's own enumerable string-keyed property [key, value] pairs.
    pub fn entries(&self) -> JSResult<Vec<Entry<V>>> {
        let ctx = &self.get_ctx();
        let mut entries = Vec::new();

        // Get all enumerable property names
        let keys = self.own_keys()?;

        // Iterate through property names to get corresponding values
        for key in keys {
            if let Some(value) = self.as_value().get_property(key.clone()) {
                entries.push(Entry {
                    key: JSValue::from_raw(ctx, key),
                    value: JSValue::from_raw(ctx, value),
                });
            }
        }

        Ok(entries)
    }

    /// Returns entries with converted types
    pub fn entries_as<K, V2>(&self) -> JSResult<Vec<(K, V2)>>
    where
        K: FromJSValue<V>,
        V2: FromJSValue<V>,
    {
        self.entries()?
            .into_iter()
            .map(|entry| entry.try_into::<K, V2>())
            .collect()
    }

    /// Returns an array of a given object's own enumerable property names
    pub fn own_keys(&self) -> JSResult<Vec<V>> {
        let mut keys = Vec::new();

        // Get all enumerable property names of the object
        if let Some(obj_keys) = self.as_value().get_own_property_names() {
            keys.extend(obj_keys);
        }

        Ok(keys)
    }

    /// Returns an iterator over the object's own enumerable string-keyed property values.
    pub fn values(&self) -> JSResult<impl Iterator<Item = JSValue<V>> + '_> {
        Ok(self.entries()?.into_iter().map(|entry| entry.value))
    }

    /// Returns values with converted type
    pub fn values_as<T>(&self) -> JSResult<Vec<T>>
    where
        T: FromJSValue<V>,
    {
        let ctx = &self.get_ctx();
        self.values()?
            .map(|v| T::from_js_value(ctx, v.into_value()))
            .collect()
    }

    /// Returns an iterator over the object's own enumerable string-keyed property names.
    pub fn keys(&self) -> JSResult<impl Iterator<Item = JSValue<V>> + '_> {
        Ok(self.entries()?.into_iter().map(|entry| entry.key))
    }

    /// Returns keys with converted type
    pub fn keys_as<K>(&self) -> JSResult<Vec<K>>
    where
        K: FromJSValue<V>,
    {
        let ctx = &self.get_ctx();
        self.keys()?
            .map(|k| K::from_js_value(ctx, k.into_value()))
            .collect()
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
