use crate::{JSContext, JSValue, JSValueRaw};
use std::default::Default;

pub trait JSValueInto<T>: JSValueRaw + Sized {
    fn into_rust(value: JSValue<Self>) -> Option<T>
    where
        T: Default;
}

pub trait JSValueFrom<T>: JSValueRaw + Sized {
    fn from_rust(ctx: &JSContext<Self::Context>, val: T) -> Self;
}
