use crate::{IntoJSValue, JSClass, JSResult, JSValueConversion, JSValueImpl, RustyJSError};

mod parameter;
pub use parameter::{
    ArgThis, FromParams, JSParameterType, Optional, ParamsAccessor, Rest, This, ThisMut,
};

trait JSCallable<V: JSValueImpl> {
    fn call(&self, accessor: &mut ParamsAccessor<V>) -> JSResult<V>;
}

impl<V, F> JSCallable<V> for F
where
    V: JSValueImpl,
    F: Fn(&mut ParamsAccessor<V>) -> JSResult<V>,
{
    fn call(&self, accessor: &mut ParamsAccessor<V>) -> JSResult<V> {
        (self)(accessor)
    }
}

/// Container to hold rust closure/function that's callable from JS.
/// Supports various parameter types:
/// - Regular parameters (i32, String, etc.)
/// - This<T> for capturing JS `this` context
/// - Optional<T> for optional parameters
/// - Rest<T> for rest parameters
///
/// Example:
/// ```ignore
/// // Function with this context and optional parameter
/// RustFunc::new(|this: This<MyClass>, x: i32, opt: Optional<String>| {
///     // ...
/// })
///
/// // Function with rest parameters
/// RustFunc::new(|x: i32, rest: Rest<String>| {
///     // ...
/// })
/// ```
pub(crate) struct RustFunc<V: JSValueImpl> {
    func: Box<dyn JSCallable<V>>,
    required_params: u32,
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
pub trait IntoJSCallable<V: JSValueImpl, P>
where
    P: FromParams<V>,
{
    fn call(&self, accessor: &mut ParamsAccessor<V>) -> JSResult<V>;
}

impl<V: JSValueImpl> RustFunc<V> {
    pub fn new<F, P>(f: F) -> Self
    where
        F: IntoJSCallable<V, P> + 'static,
        P: FromParams<V>,
    {
        let required_params = P::param_requirements().required_count() as u32;
        let func = Box::new(move |accessor: &mut ParamsAccessor<V>| f.call(accessor))
            as Box<dyn JSCallable<V>>;
        Self {
            func,
            required_params,
        }
    }

    pub fn call(&self, accessor: &mut ParamsAccessor<V>) -> JSResult<V> {
        let num_args = accessor.args_len() as u32;
        if num_args < self.required_params {
            return Err(RustyJSError::InvalidParameter {
                expected: self.required_params,
                got: num_args,
            });
        }
        self.func.call(accessor)
    }

    pub fn parameter_required_count(&self) -> u32 {
        self.required_params
    }
}

pub struct Constructor<V: JSValueImpl>(pub(crate) RustFunc<V>);

impl<V: JSValueImpl> Constructor<V> {
    pub fn new<F, P>(f: F) -> Self
    where
        F: IntoJSCallable<V, P> + 'static,
        P: FromParams<V>,
    {
        Self(RustFunc::new(f))
    }
}

impl<V> JSClass<V> for RustFunc<V>
where
    V: JSValueConversion + 'static,
{
    const NAME: &'static str = "RustFunc";

    fn data_constructor() -> Constructor<V> {
        // RustFunction class don't need data constructor
        panic!("Never 'new RustFunc()' in JS");
    }

    fn class_setup(_class: &crate::ClassSetup<V>) {}
}

macro_rules! impl_js_callable_func {
    ($($t:ident),* $(,)?) => {
        impl<V, R, Fun $(,$t)*> IntoJSCallable<V, ($($t,)*)> for Fun
        where
            Fun: Fn($($t),*) -> R,
            V: JSValueImpl,
            ($($t,)*): FromParams<V>,
            R: IntoJSValue<V>,
        {
            fn call(&self, accessor: &mut ParamsAccessor<V>) -> JSResult<V>  {
                let params = <($($t,)*)>::from_params(accessor)?;
                #[allow(non_snake_case)]
                let ($($t,)*) = params;
                let result = (self)($($t),*);
                Ok(result.into_js_value(accessor.context()))
            }
        }
    };
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
