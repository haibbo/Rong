use super::{JSValue, JSValueImpl};

// help trait contains conversion trait bound
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

/// FromJSValue trait for converting JSValue to native Rust types
/// Follow Rust's From/Into pattern for consistent type conversion
pub trait FromJSValue<'ctx, V>: Sized
where
    V: JSValueImpl,
{
    fn from_js(value: JSValue<'ctx, V>) -> Result<Self, String>;
}

pub trait JSValueInto<T> {
    fn js_into(self) -> Result<T, String>;
}

// Implement automatic JSValueInto derivation from FromJSValue
impl<'ctx, V, T> JSValueInto<T> for JSValue<'ctx, V>
where
    V: JSValueImpl,
    T: FromJSValue<'ctx, V>,
{
    fn js_into(self) -> Result<T, String> {
        T::from_js(self)
    }
}

pub trait ToJSValue<V>
where
    V: JSValueImpl,
{
    fn to_js_value(self, ctx: &V::Context) -> V;
}
