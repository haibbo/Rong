use crate::{JSContext, JSContextImpl, JSResult, RustyJSError};
use std::fmt;

mod convert;
pub use convert::*;

mod exception;
pub use exception::*;

mod valuetype;
pub use valuetype::{JSTypeOf, JSValueType};

mod object;
pub use object::*;

mod array;
pub use array::*;

mod array_buffer;
pub use array_buffer::*;

mod typed_array;
pub use typed_array::*;

mod function;
pub use function::*;

pub trait JSValueImpl: Clone {
    /// the JS engine specific type of JavaScript Value
    type RawValue: Copy;

    /// Associates with a type that implements JSContextImpl
    /// This represents the context wrapper type (e.g. QJSContext)
    type Context: JSContextImpl<Value = Self>;

    /// Create a JSValue from borrowed raw parts, increasing reference count to ensure safety.
    /// Used for values received from JS engine callbacks or external sources.
    fn from_borrowed_raw(
        ctx: <Self::Context as JSContextImpl>::RawContext,
        value: Self::RawValue,
    ) -> Self;

    /// Create a JSValue from owned raw parts without reference counting.
    /// Used for values newly created by Rust code that we own directly.
    fn from_owned_raw(
        ctx: <Self::Context as JSContextImpl>::RawContext,
        value: Self::RawValue,
    ) -> Self;

    /// Consumes the ownship and returns the FFI level of JSValue but stop triggering drop.
    /// This API should be used when engine API needs the ownshipe of JS variable
    fn into_raw_value(self) -> Self::RawValue;

    fn as_raw_value(&self) -> &Self::RawValue;
    fn as_raw_context(&self) -> &<Self::Context as JSContextImpl>::RawContext;
}

pub struct JSValue<V: JSValueImpl> {
    inner: V,
}

impl<V: JSValueImpl> Clone for JSValue<V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<V> JSValue<V>
where
    V: JSValueImpl,
{
    pub(crate) fn from_raw(_ctx: &JSContext<V::Context>, value: V) -> Self {
        Self { inner: value }
    }

    pub(crate) fn as_value(&self) -> &V {
        &self.inner
    }

    pub(crate) fn into_value(self) -> V {
        self.inner
    }

    /// Get the context associated with this JSValue
    pub fn get_ctx(&self) -> JSContext<V::Context> {
        JSContext::from_borrowed_raw_ptr(self.as_value().as_raw_context())
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
        let value = V::from((ctx.as_ref(), val));
        JSValue::from_raw(ctx, value)
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
        let value = V::from((ctx.as_ref(), ()));
        JSValue::from_raw(ctx, value)
    }
}

impl<V> FromJSValue<V> for JSValue<V>
where
    V: JSValueImpl,
{
    fn from_js_value(ctx: &JSContext<V::Context>, value: V) -> JSResult<Self> {
        Ok(JSValue::from_raw(ctx, value))
    }
}

impl<V> IntoJSValue<V> for JSValue<V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, _ctx: &JSContext<V::Context>) -> V {
        self.into_value()
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
                if unsafe { $to_fn(*self.as_raw_context(), *self.as_raw_value(), &mut result) } < 0
                {
                    #[cfg(debug_assertions)]
                    println!(
                        "Failed convert from {} to {}",
                        std::any::type_name::<$target>(),
                        std::any::type_name::<$out_type>()
                    );

                    Err(RustyJSError::ConvertError(
                        std::any::type_name::<$target>(),
                        std::any::type_name::<$out_type>(),
                    ))
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
                let ctx = t.0.as_raw();
                let raw = unsafe { $create_fn(*ctx, t.1) };
                Self::from_owned_raw(*ctx, raw)
            }
        }
    };

    ($target:ty, $type:ty, $create_fn:expr, $to_fn:expr) => {
        impl_js_converter!($target, $type, $type, $create_fn, $to_fn);
    };
}

// blanket implementing.
impl<V: JSValueImpl> crate::function::JSParameterType for JSValue<V> {}

impl<V> fmt::Display for JSValue<V>
where
    V: JSTypeOf + JSValueConversion,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.type_of() {
            JSValueType::Boolean => {
                if let Ok(val) = self.clone().try_into::<bool>() {
                    write!(f, "{}", val)
                } else {
                    write!(f, "boolean")
                }
            }
            JSValueType::Number => {
                if let Ok(val) = self.clone().try_into::<f64>() {
                    write!(f, "{}", val)
                } else {
                    write!(f, "number")
                }
            }
            JSValueType::String => {
                if let Ok(val) = self.clone().try_into::<String>() {
                    write!(f, "{}", val)
                } else {
                    write!(f, "string")
                }
            }
            JSValueType::Undefined => write!(f, "undefined"),
            JSValueType::Null => write!(f, "null"),
            JSValueType::BigInt => write!(f, "bigint"),
            JSValueType::Object => write!(f, "object"),
            JSValueType::Array => write!(f, "array"),
            JSValueType::Function => write!(f, "function"),
            JSValueType::Constructor => write!(f, "constructor"),
            JSValueType::Promise => write!(f, "promise"),
            JSValueType::Symbol => write!(f, "symbol"),
            JSValueType::Error => write!(f, "error"),
            JSValueType::Exception => write!(f, "exception"),
            JSValueType::Unknown => write!(f, "unknown"),
        }
    }
}

impl<V> fmt::Debug for JSValue<V>
where
    V: JSTypeOf + JSValueConversion,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSValue({})", self)
    }
}
