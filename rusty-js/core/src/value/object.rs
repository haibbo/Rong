use crate::{
    FromJSValue, JSContext, JSResult, JSTypeOf, JSValue, JSValueConversion, JSValueImpl,
    RustyJSError,
};
use std::ops::Deref;
use std::fmt;

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

/// caller should make sure V is object
impl<V: JSValueImpl> From<(V::Context, V)> for JSObject<V> {
    fn from(parts: (V::Context, V)) -> Self {
        let jsvalue: JSValue<V> = parts.into();
        jsvalue.into()
    }
}

impl<V> FromJSValue<V> for JSObject<V>
where
    V: JSTypeOf,
{
    fn from_js_value(ctx: &V::Context, value: V) -> JSResult<Self> {
        if value.is_object() {
            Ok(JSValue::from_raw_parts(ctx.clone(), value).into())
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
    fn into_js_value(self, _ctx: &V::Context) -> V {
        self.0.into_inner()
    }
}

pub trait JSObjectOps: JSValueConversion + JSTypeOf {
    /// if failed, it needs to return EXCEPTION
    fn new_object(ctx: &Self::Context) -> Self;

    /// if failed, it needs to return EXCEPTION
    /// constructor represents JS Class
    fn make_object<T>(ctx: &Self::Context, constructor: Self, data: *mut T) -> Self;

    /// get private data saved in object by make_object
    fn get_opaque<T>(&self) -> *mut T;

    fn del_property(&self, key: Self) -> bool;
    fn has_property(&self, key: Self) -> bool;
    fn set_property(&self, key: Self, value: Self) -> bool;

    fn set_prototype(&self, prototype: Self) -> bool;

    fn define_property(
        &self,
        key: Self,
        value: Self,
        getter: Self,
        setter: Self,
        attributes: PropertyAttributes,
    ) -> bool;

    /// if failed, it needs to return EXCEPTION
    fn get_property(&self, key: Self) -> Option<Self>;
}

impl<V> JSObject<V>
where
    V: JSObjectOps,
{
    /// new a general object
    pub fn new(ctx: &JSContext<V::Context>) -> Self {
        let value = V::new_object(ctx.as_ref());
        JSValue::new(ctx, value).into()
    }

    pub(crate) fn into_inner(self) -> V {
        self.0.into_inner()
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
        let key = k.into().into_key(self.as_ctx());
        self.as_inner()
            .set_property(key, kv.into_js_value(self.as_ctx()))
    }

    pub fn del<'a, K>(&'a self, k: K) -> bool
    where
        K: Into<PropertyKey<'a>>,
    {
        let key = k.into().into_key(self.as_ctx());
        self.as_inner().del_property(key)
    }

    pub fn has<'a, K>(&self, k: K) -> bool
    where
        K: Into<PropertyKey<'a>>,
    {
        let key = k.into().into_key(self.as_ctx());
        self.as_inner().has_property(key)
    }

    pub fn get<'a, K, T>(&'a self, k: K) -> JSResult<T>
    where
        K: Into<PropertyKey<'a>>,
        T: FromJSValue<V>,
    {
        let key = k.into().into_key(self.as_ctx());
        self.as_inner()
            .get_property(key)
            .ok_or(RustyJSError::PropertyNotFound) // check existence firstly
            .and_then(|value| T::from_js_value(self.as_ctx(), value))
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
