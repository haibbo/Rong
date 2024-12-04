use crate::{JSContext, JSValue, JSValueImpl};

pub struct JSObject<'ctx, V: JSValueImpl>(JSValue<'ctx, V>);

impl<'ctx, V> From<JSValue<'ctx, V>> for JSObject<'ctx, V>
where
    V: JSValueImpl,
{
    fn from(v: JSValue<'ctx, V>) -> Self {
        JSObject(v)
    }
}

pub trait JSObjectOps<'ctx>
where
    Self: JSValueImpl,
{
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

/// TODO: provide trait to convert T to Key
impl<'ctx, V> JSObject<'ctx, V>
where
    V: JSValueImpl + JSObjectOps<'ctx>,
{
    /// new a general object
    pub fn new(ctx: &'ctx JSContext<V::Context>) -> Self {
        let value = V::new_object(&ctx.inner);
        JSValue::new(ctx, value).into()
    }

    pub fn as_value(&self) -> &JSValue<'ctx, V> {
        &self.0
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

    pub fn set(&self, key: JSValue<'ctx, V>, value: JSValue<'ctx, V>) -> bool {
        self.0.inner.set_property(key.inner, value.inner)
    }

    pub fn del(&self, key: JSValue<'ctx, V>) -> bool {
        self.0.inner.del_property(key.inner)
    }

    pub fn has(&self, key: JSValue<'ctx, V>) -> bool {
        self.0.inner.has_property(key.inner)
    }

    pub fn get(&self, key: JSValue<'ctx, V>) -> JSValue<'ctx, V> {
        let value = self.0.inner.get_property(key.inner);
        JSValue::new(key.ctx, value)
    }
}
