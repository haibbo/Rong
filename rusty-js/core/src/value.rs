use crate::{JSContext, JSContextImpl, JSResult, RustyJSError};
use std::ops::Deref;

mod convert;
pub use convert::*;

mod exception;
pub use exception::*;

mod valuetype;
pub use valuetype::{JSTypeOf, ValueType};

mod object;
pub use object::*;

mod function;
pub use function::*;

pub trait JSValueImpl: Clone {
    /// the JS engine specific type of JavaScript Value
    type FfiValue: Copy;

    /// Associates with a type that implements JSContextImpl
    /// This represents the context wrapper type (e.g. QJSContext)
    type Context: JSContextImpl<Value = Self>;

    /// In callback context, generally, all JSValue variables are from JS engine,
    /// in order to make Rust lifetime and ownship works, these variables should
    /// be increased referece count first, and then Rust side can drop JSValue safely.
    fn from_ffi(ctx: <Self::Context as JSContextImpl>::FfiContext, value: Self::FfiValue) -> Self;

    /// almost the same as from_ffi, but without referece count increased, it's used when varialbe
    /// are returned from New JS variable function, such as eval, JS_NewObject
    fn from_parts(ctx: <Self::Context as JSContextImpl>::FfiContext, value: Self::FfiValue)
        -> Self;

    /// Consumes the ownship and returns the FFI level of JSValue but stop triggering drop.
    /// This API should be used when engine API needs the ownshipe of JS variable
    fn into_ffi_value(self) -> Self::FfiValue;

    fn as_ffi_value(&self) -> &Self::FfiValue;
    fn as_ffi_context(&self) -> &<Self::Context as JSContextImpl>::FfiContext;
}

pub struct JSValue<V: JSValueImpl> {
    inner: V,
    ctx: V::Context,
}

impl<V: JSValueImpl> Clone for JSValue<V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            ctx: self.ctx.clone(),
        }
    }
}

impl<V: JSValueImpl> From<(V::Context, V)> for JSValue<V> {
    fn from(parts: (V::Context, V)) -> Self {
        Self::from_raw_parts(parts.0, parts.1)
    }
}

impl<V> JSValue<V>
where
    V: JSValueImpl,
{
    pub(crate) fn new(ctx: &JSContext<V::Context>, value: V) -> Self {
        Self {
            inner: value,
            ctx: ctx.deref().clone(),
        }
    }

    pub(crate) fn with_value(&self, value: V) -> Self {
        Self {
            inner: value,
            ctx: self.ctx.clone(),
        }
    }

    pub fn from_raw_parts(ctx: V::Context, value: V) -> Self {
        Self { inner: value, ctx }
    }

    pub(crate) fn as_inner(&self) -> &V {
        &self.inner
    }

    pub(crate) fn into_inner(self) -> V {
        self.inner
    }

    pub(crate) fn as_ctx(&self) -> &V::Context {
        &self.ctx
    }
}

impl<V> JSValue<V>
where
    V: JSValueImpl,
{
    /// Converts  Rust value into a `JSValue`.
    pub fn from<T>(ctx: &JSContext<V::Context>, val: T) -> Self
    where
        V: for<'a> From<(&'a V::Context, T)>,
    {
        let value = V::from((&ctx.inner, val));
        JSValue::new(ctx, value)
    }

    /// Try to converts JSValue to Rust value
    pub fn try_into<T>(self) -> JSResult<T>
    where
        V: TryInto<T, Error = RustyJSError>,
        T: Default,
    {
        self.inner.try_into()
    }

    /// create JS UNDEFINED Value
    pub fn undefined(ctx: &JSContext<V::Context>) -> Self
    where
        V: for<'a> From<(&'a V::Context, ())>,
    {
        let value = V::from((&ctx.inner, ()));
        JSValue::new(ctx, value)
    }
}

impl<V: JSTypeOf> JSValue<V> {
    pub fn as_object(&self) -> Option<&JSObject<V>> {
        self.is_object().map(|_| {
            // it's safe, because JSObject is just wrapper of JSValue
            unsafe { std::mem::transmute(self) }
        })
    }
}

impl<V> FromJSValue<V> for JSValue<V>
where
    V: JSValueImpl,
{
    fn from_js_value(ctx: &V::Context, value: V) -> JSResult<Self> {
        Ok(JSValue::from_raw_parts(ctx.clone(), value))
    }
}

impl<V> IntoJSValue<V> for JSValue<V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, _ctx: &V::Context) -> V {
        self.into_inner()
    }
}

#[macro_export]
macro_rules! impl_js_converter {
    ($target:ty, $in_type:ty, $out_type:ty, $create_fn:expr, $to_fn:expr) => {
        impl TryInto<$out_type> for $target
        where
            $target: JSValueImpl,
        {
            type Error = RustyJSError;
            fn try_into(self) -> Result<$out_type, Self::Error> {
                let mut result: $out_type = Default::default();
                if unsafe { $to_fn(*self.as_ffi_context(), *self.as_ffi_value(), &mut result) } < 0
                {
                    Err(RustyJSError::ConvertError(
                        std::any::type_name::<$out_type>(),
                    ))
                } else {
                    Ok(result)
                }
            }
        }

        impl<T> From<(&T, $in_type)> for $target
        where
            T: JSContextImpl<FfiContext = <$target as JSFfiContext>::FfiContext>,
            $target: JSValueImpl<Context = T>,
        {
            fn from(t: (&T, $in_type)) -> Self {
                let ctx = t.0.to_ffi();
                let raw = unsafe { $create_fn(ctx, t.1) };
                Self::from_parts(ctx, raw)
            }
        }
    };

    ($target:ty, $type:ty, $create_fn:expr, $to_fn:expr) => {
        impl_js_converter!($target, $type, $type, $create_fn, $to_fn);
    };
}
