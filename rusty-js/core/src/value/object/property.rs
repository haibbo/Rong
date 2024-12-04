use crate::{JSContext, JSValueImpl};

pub trait IntoPropertyKey<'ctx, V: JSValueImpl> {
    fn into_key(self, ctx: &'ctx JSContext<V::Context>) -> V;
}

macro_rules! impl_into_property_key {
    ($($type:ty),*) => {
        $(
            impl<'ctx, V> IntoPropertyKey<'ctx, V> for $type
            where
                V: JSValueImpl,
                V: for<'a> From<(&'a V::Context, Self)>,
            {
                fn into_key(self, ctx: &'ctx JSContext<V::Context>) -> V {
                    (&ctx.inner, self).into()
                }
            }
        )*
    };
}

//String implement: Deref<Target = str>
impl_into_property_key!(i32, u32, i64, u64, &str, &String);

pub trait IntoPropertyValue<'ctx, V: JSValueImpl> {
    fn into_value(self, ctx: &'ctx JSContext<V::Context>) -> V;
}

macro_rules! impl_into_property_value {
    ($($type:ty),*) => {
        $(
            impl<'ctx, V> IntoPropertyValue<'ctx, V> for $type
            where
                V: JSValueImpl,
                V: for<'a> From<(&'a V::Context, Self)>,
                V::Context: 'ctx,
            {
                fn into_value(self, ctx: &'ctx JSContext<V::Context>) -> V {
                    (&ctx.inner, self).into()
                }
            }
        )*
    }
}

impl_into_property_value!(bool, i32, u32, i64, u64, f64, &str, &String);
