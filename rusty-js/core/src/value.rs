use crate::{JSContext, JSContextKind};

mod convert;
pub use convert::{JSValueFrom, JSValueInto};

mod valuetype;
pub use valuetype::{JSTypeOf, ValueType};

pub trait JSValueKind: JSTypeOf {
    // raw JS Value type
    type Raw: Copy;
    type Context: JSContextKind;

    fn new(ctx: &JSContext<Self::Context>, raw: Self::Raw) -> Self;
    fn as_raw(&self) -> &Self::Raw;
}

pub struct JSValue<'ctx, V: JSValueKind> {
    raw: V,
    ctx: &'ctx JSContext<V::Context>,
}

impl<'ctx, V: JSValueKind> JSValue<'ctx, V> {
    pub fn as_ctx(&self) -> &'ctx JSContext<V::Context> {
        self.ctx
    }

    pub fn new(ctx: &'ctx JSContext<V::Context>, raw: V) -> Self {
        Self { raw, ctx }
    }

    pub fn as_raw(&self) -> &V::Raw {
        self.raw.as_raw()
    }

    pub fn get_raw(&self) -> V::Raw {
        *self.as_raw()
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
                if unsafe { $to_fn(value.as_ctx().get_raw(), value.get_raw(), &mut result) } < 0 {
                    None
                } else {
                    Some(result)
                }
            }
        }

        impl JSValueFrom<$in_type> for $target {
            fn from_rust(ctx: &JSContext<Self::Context>, val: $in_type) -> Self {
                let raw = unsafe { $create_fn(ctx.get_raw(), val) };
                Self::new(ctx, raw)
            }
        }
    };

    ($target:ty, $type:ty, $create_fn:expr, $to_fn:expr) => {
        impl_js_converter!($target, $type, $type, $create_fn, $to_fn);
    };
}
