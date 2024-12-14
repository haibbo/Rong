use super::{JSValue, JSValueImpl};

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
    + TryInto<bool, Error = String>
    + TryInto<i32, Error = String>
    + TryInto<u32, Error = String>
    + TryInto<i64, Error = String>
    + TryInto<u64, Error = String>
    + TryInto<f64, Error = String>
    + TryInto<String, Error = String>
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
        + TryInto<bool, Error = String>
        + TryInto<i32, Error = String>
        + TryInto<u32, Error = String>
        + TryInto<i64, Error = String>
        + TryInto<u64, Error = String>
        + TryInto<f64, Error = String>
        + TryInto<String, Error = String>
{
}

/// Marker trait for types that are compatible with JavaScript values
pub trait JSCompatible: Sized {}

// impl JSCompatible for () {}
impl JSCompatible for i32 {}
impl JSCompatible for u32 {}
impl JSCompatible for i64 {}
impl JSCompatible for u64 {}
impl JSCompatible for f64 {}
impl JSCompatible for bool {}
impl JSCompatible for &str {}
impl JSCompatible for String {}

/// The trait that supports extract type from JSValue
/// Why from_js_value don't use V as input type ? Because it needs to
/// return JSValue, JSObject.
pub trait FromJSValue<'ctx, V>: Sized
where
    V: JSValueImpl,
{
    fn from_js_value(value: JSValue<'ctx, V>) -> Result<Self, String>;
}

/// extract rust primitive type from JSValue
impl<'ctx, V, T> FromJSValue<'ctx, V> for T
where
    V: JSValueImpl,
    V: TryInto<T, Error = String>,
    T: JSCompatible,
{
    fn from_js_value(value: JSValue<'ctx, V>) -> Result<T, String> {
        value.into_inner().try_into()
    }
}

/// convert to JS Value represented by trait JSValueImpl
pub trait IntoJSValue<'ctx, V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, ctx: &'ctx V::Context) -> V;
}

/// convert rust primitive type to JSValue
impl<'ctx, V, T> IntoJSValue<'ctx, V> for T
where
    V: JSValueImpl,
    V: From<(&'ctx V::Context, T)>,
    V::Context: 'ctx,
    T: JSCompatible,
{
    fn into_js_value(self, ctx: &'ctx V::Context) -> V {
        V::from((ctx, self))
    }
}
