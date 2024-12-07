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

pub trait JSValueInto<T> {
    fn js_into(self) -> Result<T, String>;
}

pub trait FromJSValue<'ctx, V>: Sized
where
    V: JSValueImpl,
{
    fn from_js(value: JSValue<'ctx, V>) -> Result<Self, String>;
}

impl<'a, V, T> FromJSValue<'a, V> for T
where
    JSValue<'a, V>: JSValueInto<T>,
    V: JSValueImpl,
    V::Context: 'a,
{
    fn from_js(value: JSValue<'a, V>) -> Result<Self, String> {
        value.js_into()
    }
}
