use crate::{
    Class, FromJSValue, IntoJSValue, JSClass, JSContext, JSContextImpl, JSExceptionHandler,
    JSObject, JSObjectOps, JSValueConversion, JSValueImpl,
};
use std::ops::Deref;

pub struct JSFunc<V: JSValueImpl>(JSObject<V>);

impl<V: JSValueImpl> Deref for JSFunc<V> {
    type Target = JSObject<V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> IntoJSValue<V> for JSFunc<V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, ctx: &V::Context) -> V {
        self.0.into_js_value(ctx)
    }
}

impl<V: JSObjectOps> JSFunc<V> {
    pub fn name(self, name: &str) -> Self {
        self.0.set("name", name);
        self
    }

    pub fn into_inner(self) -> V {
        self.0.into_inner()
    }
}

impl<C: JSContextImpl> JSContext<C>
where
    C::Value: JSObjectOps + 'static,
{
    pub fn register_function<F, P>(&self, f: F) -> JSFunc<C::Value>
    where
        F: IntoJSCallable<C::Value, P> + 'static,
    {
        let func = RustFunc::new(f);
        let length = func.parameter_count();
        let value = Class::get::<RustFunc<C::Value>>(&self.inner)
            .map(|class| class.instance::<RustFunc<C::Value>>(func));
        let obj = JSObject::from_js_value(&self.inner, value.unwrap()).unwrap();
        obj.set("length", length);
        JSFunc(obj)
    }
}

trait JSCallable<V: JSValueImpl> {
    fn call(&self, context: &V::Context, args: &[V]) -> Result<V, String>;
}

impl<V, F> JSCallable<V> for F
where
    V: JSValueImpl,
    F: Fn(&V::Context, &[V]) -> Result<V, String>,
{
    fn call(&self, context: &V::Context, args: &[V]) -> Result<V, String> {
        (self)(context, args)
    }
}

/// container to hold rust closure/fucntion that's callable from JS
/// example:
///
/// RustFunc::new( |x i32, y: i32, z: i32| x + y + z)
pub struct RustFunc<V: JSValueImpl> {
    func: Box<dyn JSCallable<V>>,
    parameter_count: u32,
}

/// Type parameter P is used to differentiate between function signatures with
/// different arities. It represents the parameter types as a tuple, e.g:
/// - () for no parameters
/// - (T1) for one parameter
/// - (T1,T2) for two parameters
///
/// This allows the compiler to select the correct implementation based on the
/// function's parameter types, while avoiding implementation conflicts since
/// each tuple type is distinct.
pub trait IntoJSCallable<V: JSValueImpl, P> {
    fn call(&self, context: &V::Context, args: &[V]) -> Result<V, String>;

    fn parameter_count() -> u32;
}

impl<V: JSValueImpl> RustFunc<V> {
    pub fn new<F, P>(f: F) -> Self
    where
        F: IntoJSCallable<V, P> + 'static,
    {
        let func = Box::new(move |context: &V::Context, args: &[V]| f.call(context, args))
            as Box<dyn JSCallable<V>>;
        Self {
            func,
            parameter_count: F::parameter_count(),
        }
    }

    pub(crate) fn call(&self, context: &V::Context, args: &[V]) -> Result<V, String>
    where
        V::Context: JSExceptionHandler<Value = V>,
    {
        let num = args.len() as u32;
        if num < self.parameter_count {
            return Ok(context.throw_type_error(format!(
                "Expected {} arguments, got {}",
                self.parameter_count, num
            )));
        }
        self.func.call(context, args)
    }

    pub(crate) fn parameter_count(&self) -> u32 {
        self.parameter_count
    }
}

impl<V> JSClass<V> for RustFunc<V>
where
    V: JSValueConversion + 'static,
{
    const NAME: &'static str = "RustFunc";

    fn data_constructor() -> RustFunc<V> {
        // RustFunction class don't need data constructor
        panic!("Never 'new RustFunc()' in JS");
    }
}

macro_rules! impl_js_callable_func {
    ($($t:ident),*$(,)?) => {
        impl<V, R, Fun $(,$t)*> IntoJSCallable<V, ($($t,)*)> for Fun
        where
            Fun: Fn($($t),*) -> R,
            V: JSValueImpl,
            $($t: FromJSValue<V>,)*
            R: IntoJSValue<V>,
        {
            #[allow(unused_variables)]
            fn call(&self, context: &V::Context, args: &[V]) -> Result<V, String>  {
                #[allow(unused_variables)]
                let mut __arg_index = 0;
                let result = (self)($(
                    {
                        let arg = $t::from_js_value(context, args[__arg_index].clone())?;
                        __arg_index += 1;
                        arg
                    }
                ),*);
                Ok(result.into_js_value(context))
            }

            fn parameter_count() -> u32 {
                count_idents!($($t),*)
            }
        }
    };
}

macro_rules! count_idents {
    () => (0);
    ($t:ident $(,$rest:ident)*) => (1 + count_idents!($($rest),*));
}

impl_js_callable_func!();
impl_js_callable_func!(P1);
impl_js_callable_func!(P1, P2);
impl_js_callable_func!(P1, P2, P3);
impl_js_callable_func!(P1, P2, P3, P4);
impl_js_callable_func!(P1, P2, P3, P4, P5);
impl_js_callable_func!(P1, P2, P3, P4, P5, P6);
impl_js_callable_func!(P1, P2, P3, P4, P5, P6, P7);
impl_js_callable_func!(P1, P2, P3, P4, P5, P6, P7, P8);
