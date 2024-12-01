use crate::{JSContext, JSContextKind};

mod convert;
pub use convert::{JSValueFrom, JSValueInto};

mod valuetype;
pub use valuetype::{JSTypeOf, ValueType};

pub trait JSValueKind: Clone {
    /// Raw JavaScript value type, e.g. qjs::JSValue
    type RawValue: Copy;

    /// Associates with a type that implements JSContextKind
    /// This represents the context wrapper type (e.g. QJSContext)
    type Context: JSContextKind;

    fn from_ffi(
        ctx_raw: <Self::Context as JSContextKind>::RawContext,
        value_raw: Self::RawValue,
    ) -> Self;
    fn as_raw_value(&self) -> &Self::RawValue;
    fn as_raw_context(&self) -> &<Self::Context as JSContextKind>::RawContext;
}

pub struct JSValue<'ctx, V: JSValueKind> {
    pub inner: V, // todo: remove pub
    ctx: &'ctx JSContext<V::Context>,
}

impl<'ctx, V> Clone for JSValue<'ctx, V>
where
    V: JSValueKind,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            ctx: self.ctx,
        }
    }
}

impl<'ctx, V: JSValueKind> JSValue<'ctx, V> {
    pub fn new(ctx: &'ctx JSContext<V::Context>, raw: V) -> Self {
        Self { inner: raw, ctx }
    }
}

impl<'ctx, V> JSValue<'ctx, V>
where
    V: JSValueKind,
{
    /// Converts a Rust value into a `JSValue`.
    pub fn from_rust<T>(ctx: &'ctx JSContext<V::Context>, val: T) -> Self
    where
        V: JSValueFrom<T>,
    {
        let raw_value = V::from_rust(ctx, val);
        JSValue::new(ctx, raw_value)
    }

    /// Converts a `JSValue` into a Rust value.
    pub fn into_rust<T>(self) -> Option<T>
    where
        V: JSValueInto<T>,
        T: Default,
    {
        V::into_rust(self)
    }
}

#[macro_export]
macro_rules! impl_js_converter {
    ($target:ty, $in_type:ty, $out_type:ty, $create_fn:expr, $to_fn:expr) => {
        impl JSValueInto<$out_type> for $target {
            fn into_rust(value: JSValue<Self>) -> Option<$out_type> {
                let mut result: $out_type = Default::default();
                if unsafe {
                    $to_fn(
                        *value.inner.as_raw_context(),
                        *value.inner.as_raw_value(),
                        &mut result,
                    )
                } < 0
                {
                    None
                } else {
                    Some(result)
                }
            }
        }

        impl JSValueFrom<$in_type> for $target {
            fn from_rust(ctx: &JSContext<Self::Context>, val: $in_type) -> Self {
                let raw = unsafe { $create_fn(ctx.get_raw(), val) };
                Self::from_ffi(ctx.get_raw(), raw)
            }
        }
    };

    ($target:ty, $type:ty, $create_fn:expr, $to_fn:expr) => {
        impl_js_converter!($target, $type, $type, $create_fn, $to_fn);
    };
}
