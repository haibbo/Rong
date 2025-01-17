use crate::{IntoJSValue, JSContext, JSValueImpl};

pub trait IntoJSArg<V: JSValueImpl> {
    fn push_js_arg(self, ctx: &JSContext<V::Context>, vec: &mut Vec<V>);
}

impl<V, T> IntoJSArg<V> for T
where
    V: JSValueImpl,
    T: IntoJSValue<V>,
{
    fn push_js_arg(self, ctx: &JSContext<V::Context>, vec: &mut Vec<V>) {
        vec.push(self.into_js_value(ctx));
    }
}

pub trait IntoJSArgs<V: JSValueImpl> {
    fn into_js_args(self, ctx: &JSContext<V::Context>) -> Vec<V>;
}

// Implement for tuples (including single-element tuples)
macro_rules! impl_into_js_args {
    ($($T:ident),*) => {
        impl<V, $($T),*> IntoJSArgs<V> for ($($T,)*)
        where
            V: JSValueImpl,
            $($T: IntoJSArg<V>),*
        {
            #[allow(unused_variables)]
            fn into_js_args(self, ctx: &JSContext<V::Context>) -> Vec<V>  {
                #[allow(non_snake_case)]
                let ($($T,)*) = self;
                #[allow(unused_mut)]
                let mut args = Vec::new();
                $($T.push_js_arg(ctx, &mut args);)*
                args
            }
        }
    };
}

impl_into_js_args!();
impl_into_js_args!(T1);
impl_into_js_args!(T1, T2);
impl_into_js_args!(T1, T2, T3);
impl_into_js_args!(T1, T2, T3, T4);
impl_into_js_args!(T1, T2, T3, T4, T5);
impl_into_js_args!(T1, T2, T3, T4, T5, T6);
impl_into_js_args!(T1, T2, T3, T4, T5, T6, T7);
impl_into_js_args!(T1, T2, T3, T4, T5, T6, T7, T8);
