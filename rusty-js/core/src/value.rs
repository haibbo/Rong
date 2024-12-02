use crate::{JSContext, JSContextKind};

mod valuetype;
pub use valuetype::{JSTypeOf, ValueType};

pub trait JSValueKind: Clone {
    /// Raw JavaScript value type, e.g. qjs::JSValue
    type RawValue;

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
    inner: V,
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

impl<'ctx, V> JSValue<'ctx, V>
where
    V: JSValueKind,
{
    pub fn new(ctx: &'ctx JSContext<V::Context>, value: V) -> Self {
        Self { inner: value, ctx }
    }
}

impl<'ctx, V> JSValue<'ctx, V>
where
    V: JSValueKind,
{
    /// Converts a Rust value into a `JSValue`.
    pub fn from<T>(ctx: &'ctx JSContext<V::Context>, val: T) -> Self
    where
        V: From<(&'ctx V::Context, T)>,
    {
        let value = V::from((&ctx.inner, val));
        JSValue::new(ctx, value)
    }

    /// Converts a `JSValue` into a Rust value.
    pub fn try_into<T>(self) -> Result<T, String>
    where
        V: TryInto<T, Error = String>,
        T: Default,
    {
        self.inner.try_into()
    }
}

#[macro_export]
macro_rules! impl_js_converter {
    ($target:ty, $in_type:ty, $out_type:ty, $create_fn:expr, $to_fn:expr) => {
        impl TryInto<$out_type> for $target
        where
            $target: JSValueKind,
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
            T: JSContextKind<RawContext = <$target as JSRawContext>::RawContext>,
            $target: JSValueKind<Context = T>,
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
