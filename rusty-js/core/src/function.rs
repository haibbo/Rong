use crate::{
    IntoJSValue, JSClass, JSContext, JSObjectOps, JSResult, JSValueImpl, Promise, PromiseResolver,
    RustyJSError,
};
use std::future::Future;

mod parameter;
pub use parameter::{FromParams, JSParameterType, Optional, ParamsAccessor, Rest, This, ThisMut};

trait JSCallable<V: JSValueImpl> {
    fn call(&mut self, accessor: &mut ParamsAccessor<V>) -> JSResult<V>;
}

impl<V, F> JSCallable<V> for F
where
    V: JSValueImpl,
    F: FnMut(&mut ParamsAccessor<V>) -> JSResult<V>,
{
    fn call(&mut self, accessor: &mut ParamsAccessor<V>) -> JSResult<V> {
        (self)(accessor)
    }
}

/// Container to hold rust closure/function that's callable from JS.
///
/// Supports various parameter types:
/// - Regular parameters (i32, String, etc.)
/// - This<T> for capturing JS `this` context
/// - Optional<T> for optional parameters
/// - Rest<T> for rest parameters
///
/// Example:
/// ```ignore
/// // Sync function - K will be inferred as SyncFunc
/// RustFunc::new(|x: i32| x + 1);
///
/// // Async function - K will be inferred as AsyncFunc
/// RustFunc::new(|x: i32| async move {
///     // async operation
///     x + 1
/// });
/// ```
pub(crate) struct RustFunc<V: JSValueImpl> {
    func: Box<dyn JSCallable<V>>,
    required_params: u32,
}

/// Marker types for function kinds
/// Marker type for synchronous functions.
/// Functions returning direct values will be automatically marked with this type.
pub struct SyncFunc;

/// Marker type for asynchronous functions.
/// Functions returning Future will be automatically marked with this type.
pub struct AsyncFunc;

/// Trait for converting Rust functions into JavaScript callable functions.
/// Type parameters:
/// - V: The JavaScript value type
/// - P: Parameter types tuple
/// - K: Function kind (SyncFunc or AsyncFunc), automatically inferred from function signature
pub trait IntoJSCallable<V: JSValueImpl, P, K>
where
    P: FromParams<V>,
{
    fn call(&mut self, accessor: &mut ParamsAccessor<V>) -> JSResult<V>;
}

impl<V: JSValueImpl> RustFunc<V> {
    pub fn new<F, P, K>(mut f: F) -> Self
    where
        F: IntoJSCallable<V, P, K> + 'static,
        P: FromParams<V>,
        K: 'static,
    {
        let required_params = P::param_requirements().required_count() as u32;
        let func = Box::new(move |accessor: &mut ParamsAccessor<V>| f.call(accessor))
            as Box<dyn JSCallable<V>>;
        Self {
            func,
            required_params,
        }
    }

    pub fn call(&mut self, accessor: &mut ParamsAccessor<V>) -> JSResult<V> {
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
    pub fn new<F, P, K>(f: F) -> Self
    where
        F: IntoJSCallable<V, P, K> + 'static,
        P: FromParams<V>,
        K: 'static,
    {
        Self(RustFunc::new(f))
    }
}

impl<V> JSClass<V> for RustFunc<V>
where
    V: JSValueImpl + 'static,
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
        // Sync function implementation - automatically chosen for functions returning direct values
        impl<V, R, Fun $(,$t)*> IntoJSCallable<V, ($($t,)*), SyncFunc> for Fun
        where
            Fun: FnMut($($t),*) -> R,
            V: JSValueImpl,
            ($($t,)*): FromParams<V>,
            R: IntoJSValue<V>,
        {
            fn call(&mut self, accessor: &mut ParamsAccessor<V>) -> JSResult<V>  {
                let params = <($($t,)*)>::from_params(accessor)?;
                #[allow(non_snake_case)]
                let ($($t,)*) = params;
                let result = (self)($($t),*);
                Ok(result.into_js_value(accessor.context()))
            }
        }

        // Async function implementation - automatically chosen for functions returning Future
        impl<V, R, Fun, Fut $(,$t)*> IntoJSCallable<V, ($($t,)*), AsyncFunc> for Fun
        where
            Fun: FnMut($($t),*) -> Fut,
            Fut: Future<Output = R> + 'static,
            R: IntoJSValue<V> + 'static,
            R: PromiseResolver<V>,
            V: JSValueImpl + JSObjectOps+'static,
            ($($t,)*): FromParams<V>,
        {
            fn call(&mut self, accessor: &mut ParamsAccessor<V>) -> JSResult<V>  {
                let params = <($($t,)*)>::from_params(accessor)?;
                #[allow(non_snake_case)]
                let ($($t,)*) = params;
                let future = (self)($($t),*);
                let ctx=accessor.context();
                let jsctx =JSContext::from_raw_ptr(ctx);
                Ok(Promise::from_future(&jsctx,future)?.into_js_value(ctx))
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
