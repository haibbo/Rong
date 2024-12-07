use crate::{JSContext, JSContextImpl};
use std::string::String;

mod convert;
pub use convert::*;

mod exception;
pub use exception::*;

mod valuetype;
pub use valuetype::{JSTypeOf, ValueType};

mod object;
pub use object::*;

pub trait JSValueImpl: Clone {
    /// Raw JavaScript value type, e.g. qjs::JSValue
    type RawValue: Copy;

    /// Associates with a type that implements JSContextImpl
    /// This represents the context wrapper type (e.g. QJSContext)
    type Context: JSContextImpl;

    fn from_ffi(
        ctx_raw: <Self::Context as JSContextImpl>::RawContext,
        value_raw: Self::RawValue,
    ) -> Self;
    fn as_raw_value(&self) -> &Self::RawValue;
    fn as_raw_context(&self) -> &<Self::Context as JSContextImpl>::RawContext;
}

pub struct JSValue<'ctx, V: JSValueImpl> {
    inner: V,
    ctx: &'ctx JSContext<V::Context>,
}

impl<V: JSValueImpl> Clone for JSValue<'_, V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            ctx: self.ctx,
        }
    }
}

impl<'ctx, V> JSValue<'ctx, V>
where
    V: JSValueImpl,
{
    pub(crate) fn new(ctx: &'ctx JSContext<V::Context>, value: V) -> Self {
        Self { inner: value, ctx }
    }

    pub(crate) fn as_inner(&self) -> &V {
        &self.inner
    }

    pub(crate) fn as_ctx(&self) -> &'ctx JSContext<V::Context> {
        self.ctx
    }
}

impl<'ctx, V> JSValue<'ctx, V>
where
    V: JSValueImpl,
{
    /// Converts a Rust value into a `JSValue`.
    pub fn from<T>(ctx: &'ctx JSContext<V::Context>, val: T) -> Self
    where
        V: From<(&'ctx V::Context, T)>,
    {
        let value = V::from((&ctx.inner, val));
        JSValue::new(ctx, value)
    }

    /// create JS UNDEFINED Value
    pub fn undefined(ctx: &'ctx JSContext<V::Context>) -> Self
    where
        V: From<(&'ctx V::Context, ())>,
    {
        let value = V::from((&ctx.inner, ()));
        JSValue::new(ctx, value)
    }
}

impl<'ctx, V: JSTypeOf> JSValue<'ctx, V> {
    pub fn as_object(&self) -> Option<&JSObject<'ctx, V>> {
        self.is_object().map(|_| {
            // it's safe, because JSObject is just wrapper of JSValue
            unsafe { std::mem::transmute(self) }
        })
    }
}

impl<'ctx, V> FromJSValue<'ctx, V> for ()
where
    V: JSValueConversion,
{
    fn from_js(_v: JSValue<'ctx, V>) -> Result<(), String> {
        Ok(())
    }
}

impl<'ctx, V> FromJSValue<'ctx, V> for JSValue<'ctx, V>
where
    V: JSValueImpl,
{
    fn from_js(value: JSValue<'ctx, V>) -> Result<JSValue<'ctx, V>, String> {
        Ok(value)
    }
}

impl<'ctx, V> IntoPropertyValue<'ctx, V> for JSValue<'ctx, V>
where
    V: JSValueImpl,
{
    fn into_kv(self, _ctx: &'ctx JSContext<V::Context>) -> V {
        self.inner
    }
}

#[macro_export]
macro_rules! impl_js_converter {
    ($target:ty, $in_type:ty, $out_type:ty, $create_fn:expr, $to_fn:expr) => {
        impl TryInto<$out_type> for $target
        where
            $target: JSValueImpl,
        {
            type Error = String;
            fn try_into(self) -> Result<$out_type, Self::Error> {
                let mut result: $out_type = Default::default();
                if unsafe { $to_fn(*self.as_raw_context(), *self.as_raw_value(), &mut result) } < 0
                {
                    let err = format!(
                        "Failed to convert JS Value into Rust type: {}",
                        std::any::type_name::<$out_type>()
                    );
                    Err(err)
                } else {
                    Ok(result)
                }
            }
        }

        impl<T> From<(&T, $in_type)> for $target
        where
            T: JSContextImpl<RawContext = <$target as JSRawContext>::RawContext>,
            $target: JSValueImpl<Context = T>,
        {
            fn from(t: (&T, $in_type)) -> Self {
                let ctx = *t.0.as_raw();
                let raw = unsafe { $create_fn(ctx, t.1) };
                Self::from_ffi(ctx, raw)
            }
        }
    };

    ($target:ty, $type:ty, $create_fn:expr, $to_fn:expr) => {
        impl_js_converter!($target, $type, $type, $create_fn, $to_fn);
    };
}

/// help implement JSValueInto for primitive type
/// it consumes the ownship
macro_rules! impl_from_jsvalue {
    ($($ty:ty),*) => {
        $(
            impl<'ctx, V> FromJSValue<'ctx, V> for $ty
            where
                V: JSValueConversion,
            {
                fn from_js(value: JSValue<'ctx, V>) -> Result<Self, String> {
                    value.inner.try_into()
                }
            }
        )*
    };
}

impl_from_jsvalue!(bool, i32, u32, i64, u64, f64, String);
