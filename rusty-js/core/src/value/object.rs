use crate::{JSContext, JSValue, JSValueImpl};

mod property;
pub use property::{IntoPropertyKey, IntoPropertyValue};

pub struct JSObject<'ctx, V: JSValueImpl>(JSValue<'ctx, V>);

impl<'ctx, V> From<JSValue<'ctx, V>> for JSObject<'ctx, V>
where
    V: JSValueImpl,
{
    fn from(v: JSValue<'ctx, V>) -> Self {
        JSObject(v)
    }
}

impl<'ctx, V: JSValueImpl> JSObject<'ctx, V> {
    pub fn as_value(&self) -> &JSValue<'ctx, V> {
        &self.0
    }
}

impl<'ctx, V> IntoPropertyValue<'ctx, V> for JSObject<'ctx, V>
where
    V: JSValueImpl,
{
    fn into_value(self, _ctx: &'ctx JSContext<V::Context>) -> V {
        self.0.inner
    }
}

pub trait JSObjectOps<'ctx>: JSValueImpl {
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
    fn get_property(&self, key: Self) -> Self;
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
        self.0.inner.get_opaque()
    }
}

impl<'ctx, V> JSObject<'ctx, V>
where
    V: JSObjectOps<'ctx>,
{
    pub fn set<K, KV>(&self, k: K, kv: KV) -> bool
    where
        K: IntoPropertyKey<'ctx, V>,
        KV: IntoPropertyValue<'ctx, V>,
    {
        let key = k.into_key(self.0.ctx);
        self.0.inner.set_property(key, kv.into_value(self.0.ctx))
    }

    pub fn del<K>(&self, k: K) -> bool
    where
        K: IntoPropertyKey<'ctx, V>,
    {
        let key = k.into_key(self.0.ctx);
        self.0.inner.del_property(key)
    }

    pub fn has<K>(&self, k: K) -> bool
    where
        K: IntoPropertyKey<'ctx, V>,
    {
        let key = k.into_key(self.0.ctx);
        self.0.inner.has_property(key)
    }

    pub fn get<K>(&self, k: K) -> JSValue<'ctx, V>
    where
        K: IntoPropertyKey<'ctx, V>,
    {
        let key = k.into_key(self.0.ctx);
        let value = self.0.inner.get_property(key);
        JSValue::new(self.0.ctx, value)
    }
}
