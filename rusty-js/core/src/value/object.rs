use crate::{FromJSValue, JSContext, JSTypeOf, JSValue, JSValueConversion, JSValueImpl};
use std::ops::Deref;
use std::string::String;

mod property;
pub use property::{IntoPropertyValue, PropertyKey};

pub struct JSObject<'ctx, V: JSValueImpl>(JSValue<'ctx, V>);

impl<'ctx, V> From<JSValue<'ctx, V>> for JSObject<'ctx, V>
where
    V: JSValueImpl,
{
    fn from(v: JSValue<'ctx, V>) -> Self {
        JSObject(v)
    }
}

impl<'ctx, V> FromJSValue<'ctx, V> for JSObject<'ctx, V>
where
    V: JSTypeOf,
{
    fn from_js(value: JSValue<'ctx, V>) -> Result<Self, String> {
        if value.is_object().is_some() {
            Ok(value.into())
        } else {
            Err("not an object".to_string())
        }
    }
}

impl<'ctx, V: JSValueImpl> Deref for JSObject<'ctx, V> {
    type Target = JSValue<'ctx, V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'ctx, V> IntoPropertyValue<'ctx, V> for JSObject<'ctx, V>
where
    V: JSValueImpl,
{
    fn into_kv(self, _ctx: &'ctx JSContext<V::Context>) -> V {
        self.0.inner
    }
}

pub trait JSObjectOps<'ctx>: JSValueConversion + JSTypeOf {
    /// if failed, it needs to return EXCEPTION
    fn new_object(ctx: &'ctx Self::Context) -> Self;

    /// if failed, it needs to return EXCEPTION
    /// constructor represents JS Class
    /// TODO: change constructor's type TO JSFunc
    fn make_object<T>(ctx: &'ctx Self::Context, constructor: Self, data: *mut T) -> Self;

    /// get private data saved in object by make_object
    fn get_opaque<T>(&self) -> *mut T;

    fn del_property(&self, key: Self) -> bool;
    fn has_property(&self, key: Self) -> bool;
    fn set_property(&self, key: Self, value: Self) -> bool;

    /// if failed, it needs to return EXCEPTION
    fn get_property(&self, key: Self) -> Option<Self>;
}

impl<'ctx, V> JSObject<'ctx, V>
where
    V: JSObjectOps<'ctx>,
{
    /// new a general object
    pub fn new(ctx: &'ctx JSContext<V::Context>) -> Self {
        let value = V::new_object(&ctx.inner);
        JSValue::new(ctx, value).into()
    }

    /// new object instance of Class with private data
    pub fn make<T>(
        ctx: &'ctx JSContext<V::Context>,
        construct: JSValue<'ctx, V>,
        opaque: *mut T,
    ) -> Self {
        let value = V::make_object(&ctx.inner, construct.inner, opaque);
        Self(JSValue::new(ctx, value))
    }

    /// get private data
    pub fn get_opaque<T>(&self) -> *mut T {
        self.as_inner().get_opaque()
    }
}

impl<'ctx, V> JSObject<'ctx, V>
where
    V: JSObjectOps<'ctx>,
{
    pub fn set<K, KV>(&self, k: K, kv: KV) -> bool
    where
        K: Into<PropertyKey<'ctx>>,
        KV: IntoPropertyValue<'ctx, V>,
    {
        let key = k.into().into_key(self.as_ctx());
        self.as_inner().set_property(key, kv.into_kv(self.0.ctx))
    }

    pub fn del<K>(&self, k: K) -> bool
    where
        K: Into<PropertyKey<'ctx>>,
    {
        let key = k.into().into_key(self.as_ctx());
        self.as_inner().del_property(key)
    }

    pub fn has<K>(&self, k: K) -> bool
    where
        K: Into<PropertyKey<'ctx>>,
    {
        let key = k.into().into_key(self.as_ctx());
        self.as_inner().has_property(key)
    }

    pub fn get<K, T>(&self, k: K) -> Result<T, String>
    where
        K: Into<PropertyKey<'ctx>>,
        T: FromJSValue<'ctx, V>,
    {
        let key = k.into().into_key(self.as_ctx());
        self.as_inner()
            .get_property(key)
            .ok_or_else(|| String::from("Property not found")) // check existence firstly
            .and_then(|value| T::from_js(JSValue::new(self.0.ctx, value)))
    }
}
