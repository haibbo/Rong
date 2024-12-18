use crate::{
    FromJSValue, IntoJSValue, JSContextImpl, JSExceptionHandler, JSObject, JSValueConversion,
    JSValueImpl,
};
use std::ops::Deref;

pub struct JSFunc<'ctx, V: JSValueImpl>(JSObject<'ctx, V>);

impl<'ctx, V: JSValueImpl> Deref for JSFunc<'ctx, V> {
    type Target = JSObject<'ctx, V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> IntoJSValue<V> for JSFunc<'_, V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, ctx: &V::Context) -> V {
        self.0.into_js_value(ctx)
    }
}

pub trait RustCallable<V, C> {
    fn call(&self, context: &C, args: &[V]) -> Result<V, String>;
}

impl<V, C, F> RustCallable<V, C> for F
where
    F: Fn(&C, &[V]) -> Result<V, String>,
{
    fn call(&self, context: &C, args: &[V]) -> Result<V, String> {
        (self)(context, args)
    }
}

/// container to hold rust closure/fucntion that's callable from JS
/// example:
///
/// RustFunc::new( |x i32, y: i32, z: i32| x + y + z)
pub struct RustFunc<V, C>(Box<dyn RustCallable<V, C>>);

impl<V, C> RustCallable<V, C> for RustFunc<V, C> {
    fn call(&self, context: &C, args: &[V]) -> Result<V, String> {
        self.0.call(context, args)
    }
}

/// Type parameter P is used to differentiate between function signatures with
/// different arities. It represents the parameter types as a tuple, e.g:
/// - () for no parameters
/// - (T1,) for one parameter
/// - (T1,T2) for two parameters
///
/// This allows the compiler to select the correct implementation based on the
/// function's parameter types, while avoiding implementation conflicts since
/// each tuple type is distinct.
pub trait IntoRustCallable<V, C, P> {
    fn call(&self, context: &C, args: &[V]) -> Result<V, String>;
}

impl<V, C> RustFunc<V, C> {
    pub fn new<F, P>(f: F) -> Self
    where
        F: IntoRustCallable<V, C, P> + 'static,
    {
        let func = Box::new(move |context: &C, args: &[V]| f.call(context, args))
            as Box<dyn RustCallable<V, C>>;
        Self(func)
    }
}

macro_rules! impl_rust_callable_func {
     () => {
        impl<V, C, R, Fun> IntoRustCallable<V, C, ()> for Fun
        where
            Fun: Fn() -> R,
            V: JSValueImpl<Context = C>,
            C: JSContextImpl,
            R: IntoJSValue<V>,
        {
            fn call(&self, context: &C, _args: &[V]) ->Result<V, String> {  // _ make lint happy
                let result = (self)();
                Ok(result.into_js_value(context))
            }
        }
    };

    ($($t:ident),*$(,)?) => {
        impl<V, C, R, Fun $(,$t)*> IntoRustCallable<V, C, ($($t,)*)> for Fun
        where
            Fun: Fn($($t),*) -> R,
            V: JSValueImpl<Context = C> + JSValueConversion,
            C: JSContextImpl + JSExceptionHandler<Value=V>,
            $($t: FromJSValue<V>,)*
            R: IntoJSValue<V>,
        {
            fn call(&self, context: &C, args: &[V]) -> Result<V, String>  {
                let expected = count_idents!($($t),*);
                if args.len() < expected {
                    // TODO: improve error handler
                    return Ok(context.throw_type_error(&format!(
                        "Expected {} arguments, got {}",
                        expected,
                        args.len())
                    ));
                }

                let result = (self)($(
                    $t::from_js_value(context, args.get(count_idents!($t))
                        .ok_or("Missing argument")?.clone())?
                ),*);
                Ok(result.into_js_value(context))
            }
        }
    };
}

macro_rules! count_idents {
    () => (0);
    ($t:ident $(,$rest:ident)*) => (1 + count_idents!($($rest),*));
}

impl_rust_callable_func!();
impl_rust_callable_func!(P1);
impl_rust_callable_func!(P1, P2);
impl_rust_callable_func!(P1, P2, P3);
impl_rust_callable_func!(P1, P2, P3, P4);
impl_rust_callable_func!(P1, P2, P3, P4, P5);
impl_rust_callable_func!(P1, P2, P3, P4, P5, P6);
impl_rust_callable_func!(P1, P2, P3, P4, P5, P6, P7);
impl_rust_callable_func!(P1, P2, P3, P4, P5, P6, P7, P8);
