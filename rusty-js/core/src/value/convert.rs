use crate::{JSContext, JSValue, JSValueKind};
use std::default::Default;

pub trait JSValueInto<T>: JSValueKind + Sized {
    fn into_rust(value: JSValue<Self>) -> Option<T>
    where
        T: Default;
}

pub trait JSValueFrom<T>: JSValueKind + Sized {
    fn from_rust(ctx: &JSContext<Self::Context>, val: T) -> Self;
}
