use crate::{JSContext, JSValueImpl};

pub trait IntoPropertyKey<'ctx, V: JSValueImpl> {
    fn into_key(self, ctx: &'ctx JSContext<V::Context>) -> V;
}

macro_rules! impl_property_key {
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
impl_property_key!(i32, u32, i64, u64, &str, &String);

pub trait IntoPropertyValue<'ctx, V: JSValueImpl> {
    fn into_value(self, ctx: &'ctx JSContext<V::Context>) -> V;
}

impl<'ctx, V, T> IntoPropertyValue<'ctx, V> for T
where
    V: JSValueImpl,
    V: From<(&'ctx V::Context, T)>,
    V::Context: 'ctx,
{
    fn into_value(self, ctx: &'ctx JSContext<V::Context>) -> V {
        (&ctx.inner, self).into()
    }
}
