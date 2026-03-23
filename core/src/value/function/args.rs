use crate::{IntoJSValue, JSContext, JSValueImpl, function::JSParameterType};
use smallvec::SmallVec;

pub type JSArgsVec<V> = SmallVec<[V; 4]>;

pub trait IntoJSArg<V: JSValueImpl> {
    fn push_js_arg(self, ctx: &JSContext<V::Context>, vec: &mut JSArgsVec<V>);
}

// Generic implementation for all types that implement JSParameterType
impl<V, T> IntoJSArg<V> for T
where
    V: JSValueImpl,
    T: IntoJSValue<V>,
    T: JSParameterType,
{
    fn push_js_arg(self, ctx: &JSContext<V::Context>, vec: &mut JSArgsVec<V>) {
        vec.push(<T as IntoJSValue<V>>::into_js_value(self, ctx).into_value());
    }
}

// Special handling for Rest parameters
impl<V, T> IntoJSArg<V> for Vec<T>
where
    V: JSValueImpl,
    T: IntoJSValue<V>,
{
    fn push_js_arg(self, ctx: &JSContext<V::Context>, vec: &mut JSArgsVec<V>) {
        vec.extend(
            self.into_iter()
                .map(|item| <T as IntoJSValue<V>>::into_js_value(item, ctx).into_value()),
        );
    }
}

pub trait IntoJSArgs<V: JSValueImpl> {
    fn into_js_args(self, ctx: &JSContext<V::Context>) -> JSArgsVec<V>;
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
            fn into_js_args(self, ctx: &JSContext<V::Context>) -> JSArgsVec<V>  {
                #[allow(non_snake_case)]
                let ($($T,)*) = self;
                #[allow(unused_mut)]
                let mut args = JSArgsVec::new();
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
