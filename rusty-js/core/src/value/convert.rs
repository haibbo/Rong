use super::JSValueImpl;
use crate::{JSResult, RustyJSError};

/// The conversion between Rust primitive types, which implement the `JSCompatible`
/// marker trait, and `JSValue` is facilitated using the standard `TryInto` and
/// `From` traits.
///
/// The `impl_js_converter` macro simplifies the implementation of these conversions.
///
/// Since `TryInto` and `From` are implemented for local types like `QJSValue` in
/// the `quickjs` crate, the orphan rule does not apply here.
///
/// We introduce `FromJSValue` and `IntoJSValue` traits for application-level use
/// cases:
/// - `FromJSValue` is particularly useful for returning generic types from operations
///   such as `eval` and `JSObject.get`.
/// - `IntoJSValue` allows `JSObject.set` to accept a wider range of types that
///    can be converted into `JSValue`.
///
/// help trait contains conversion trait bound
/// it help simplify trait boud for upper caller
pub trait JSValueConversion:
    JSValueImpl
    + for<'a> From<(&'a Self::Context, ())>
    + for<'a> From<(&'a Self::Context, bool)>
    + for<'a> From<(&'a Self::Context, i32)>
    + for<'a> From<(&'a Self::Context, u32)>
    + for<'a> From<(&'a Self::Context, i64)>
    + for<'a> From<(&'a Self::Context, u64)>
    + for<'a> From<(&'a Self::Context, f64)>
    + for<'a> From<(&'a Self::Context, &'a str)>
    + TryInto<bool, Error = RustyJSError>
    + TryInto<i32, Error = RustyJSError>
    + TryInto<u32, Error = RustyJSError>
    + TryInto<i64, Error = RustyJSError>
    + TryInto<u64, Error = RustyJSError>
    + TryInto<f64, Error = RustyJSError>
    + TryInto<String, Error = RustyJSError>
{
}

// Automatically implement types that satisfy trait bound
impl<T> JSValueConversion for T where
    T: JSValueImpl
        + for<'a> From<(&'a T::Context, ())>
        + for<'a> From<(&'a T::Context, bool)>
        + for<'a> From<(&'a T::Context, i32)>
        + for<'a> From<(&'a T::Context, u32)>
        + for<'a> From<(&'a T::Context, i64)>
        + for<'a> From<(&'a T::Context, u64)>
        + for<'a> From<(&'a T::Context, f64)>
        + for<'a> From<(&'a T::Context, &'a str)>
        + TryInto<bool, Error = RustyJSError>
        + TryInto<i32, Error = RustyJSError>
        + TryInto<u32, Error = RustyJSError>
        + TryInto<i64, Error = RustyJSError>
        + TryInto<u64, Error = RustyJSError>
        + TryInto<f64, Error = RustyJSError>
        + TryInto<String, Error = RustyJSError>
{
}

/// Marker trait for types that are compatible with JavaScript values
pub trait JSCompatible: Sized {}

impl JSCompatible for i32 {}
impl JSCompatible for u32 {}
impl JSCompatible for i64 {}
impl JSCompatible for u64 {}
impl JSCompatible for f64 {}
impl JSCompatible for bool {}

/// The trait that supports extract type from JSValue
/// Why from_js_value don't use V as input type ? Because it needs to
/// return JSValue, JSObject.
pub trait FromJSValue<V>: Sized
where
    V: JSValueImpl,
{
    fn from_js_value(ctx: &V::Context, value: V) -> JSResult<Self>;
}

/// extract rust primitive type from JSValue
impl<V, T> FromJSValue<V> for T
where
    V: JSValueImpl,
    V: TryInto<T, Error = RustyJSError>,
    T: JSCompatible,
{
    fn from_js_value(_ctx: &V::Context, value: V) -> JSResult<Self> {
        value.try_into()
    }
}

impl<V> FromJSValue<V> for ()
where
    V: JSValueConversion,
{
    fn from_js_value(_ctx: &V::Context, _value: V) -> JSResult<Self> {
        Ok(())
    }
}

impl<V> FromJSValue<V> for String
where
    V: JSValueImpl,
    V: TryInto<String, Error = RustyJSError>,
{
    fn from_js_value(_ctx: &V::Context, value: V) -> JSResult<Self> {
        value.try_into()
    }
}

/// convert to JS Value represented by trait JSValueImpl
pub trait IntoJSValue<V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, ctx: &V::Context) -> V;
}

impl<V> IntoJSValue<V> for &str
where
    V: JSValueImpl,
    V: for<'a> From<(&'a V::Context, &'a str)>,
{
    fn into_js_value(self, ctx: &V::Context) -> V {
        V::from((ctx, self))
    }
}

impl<V> IntoJSValue<V> for String
where
    V: JSValueImpl,
    V: for<'a> From<(&'a V::Context, &'a str)>,
{
    fn into_js_value(self, ctx: &V::Context) -> V {
        V::from((ctx, self.as_str()))
    }
}

impl<V> IntoJSValue<V> for ()
where
    V: JSValueConversion,
{
    fn into_js_value(self, ctx: &V::Context) -> V {
        V::from((ctx, self))
    }
}

/// convert rust primitive type to JSValue
impl<V, T> IntoJSValue<V> for T
where
    V: JSValueImpl,
    V: for<'a> From<(&'a V::Context, T)>,
    T: JSCompatible,
{
    fn into_js_value(self, ctx: &V::Context) -> V {
        V::from((ctx, self))
    }
}
