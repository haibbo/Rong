use crate::{JSContext, JSValueConversion};

pub enum PropertyKey<'a> {
    Int32(i32),
    Uint32(u32),
    Int64(i64),
    Uint64(u64),
    Str(&'a str),
    // Symbol(Symbol),
}

impl From<i32> for PropertyKey<'_> {
    fn from(value: i32) -> Self {
        PropertyKey::Int32(value)
    }
}

impl From<u32> for PropertyKey<'_> {
    fn from(value: u32) -> Self {
        PropertyKey::Uint32(value)
    }
}

impl From<i64> for PropertyKey<'_> {
    fn from(value: i64) -> Self {
        PropertyKey::Int64(value)
    }
}

impl From<u64> for PropertyKey<'_> {
    fn from(value: u64) -> Self {
        PropertyKey::Uint64(value)
    }
}

impl<'a> From<&'a str> for PropertyKey<'a> {
    fn from(value: &'a str) -> Self {
        PropertyKey::Str(value)
    }
}

impl<'ctx> PropertyKey<'ctx> {
    pub fn into_key<V>(self, ctx: &'ctx JSContext<V::Context>) -> V
    where
        V: JSValueConversion,
    {
        match self {
            Self::Int32(i) => (&ctx.inner, i).into(),
            Self::Uint32(i) => (&ctx.inner, i).into(),
            Self::Int64(i) => (&ctx.inner, i).into(),
            Self::Uint64(i) => (&ctx.inner, i).into(),
            Self::Str(s) => (&ctx.inner, s).into(),
        }
    }
}
