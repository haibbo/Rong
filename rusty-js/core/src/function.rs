use crate::{
    IntoJSValue, JSClass, JSObjectOps, JSResult, JSValueImpl, Promise, PromiseResolver,
    RustyJSError,
};
use std::future::Future;

mod parameter;
pub use parameter::{FromParams, JSParameterType, Optional, ParamsAccessor, Rest, This, ThisMut};

/// This trait is implemented for closures that take a mutable reference to ParamsAccessor
/// and return a JSResult. It's used for functions that can be called repeatedly.
trait JSCallable<V: JSValueImpl> {
    fn call(&mut self, accessor: &mut ParamsAccessor<V>) -> JSResult<V>;
}

/// This trait is implemented for closures that take ownership of self and ParamsAccessor
/// and return a JSResult. It's used for functions that can only be called once.
trait OnceJSCallable<V: JSValueImpl> {
    fn once(self: Box<Self>, accessor: &mut ParamsAccessor<V>) -> JSResult<V>;
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

impl<V, F> OnceJSCallable<V> for F
where
    V: JSValueImpl,
    F: FnOnce(&mut ParamsAccessor<V>) -> JSResult<V>,
{
    fn once(self: Box<Self>, accessor: &mut ParamsAccessor<V>) -> JSResult<V> {
        (*self)(accessor)
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
    func: FuncType<V>,
    required_params: u32,
}

enum FuncType<V: JSValueImpl> {
    MutFn(Box<dyn JSCallable<V>>),
    OnceFn(Option<Box<dyn OnceJSCallable<V>>>),
}

/// Marker type for synchronous functions.
/// Functions returning direct values will be automatically marked with this type.
pub struct SyncFnMut;

/// Marker type for asynchronous functions.
/// Functions returning Future will be automatically marked with this type.
pub struct AsyncFnMut;

/// Marker type for synchronous functions.
pub struct SyncFnOnce;

/// Marker type for asynchronous functions.
pub struct AsyncFnOnce;

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

/// same as IntoJSCallable but for FnOnce function or closure
pub trait IntoOnceJSCallable<V: JSValueImpl, P, K>
where
    P: FromParams<V>,
{
    fn call(self, accessor: &mut ParamsAccessor<V>) -> JSResult<V>;
}

impl<V: JSValueImpl> RustFunc<V> {
    /// Creates a new RustFunc instance for wrapping a multi-callable Rust closure/function
    pub(crate) fn new<F, P, K>(mut f: F) -> Self
    where
        F: IntoJSCallable<V, P, K> + 'static,
        P: FromParams<V>,
        K: 'static,
    {
        let required_params = P::param_requirements().required_count() as u32;
        let func = Box::new(move |accessor: &mut ParamsAccessor<V>| f.call(accessor))
            as Box<dyn JSCallable<V>>;
        Self {
            func: FuncType::MutFn(func),
            required_params,
        }
    }

    /// same as `new`, but only callable once time
    pub(crate) fn new_once<F, P, K>(f: F) -> Self
    where
        F: IntoOnceJSCallable<V, P, K> + 'static,
        P: FromParams<V>,
        K: 'static,
    {
        let required_params = P::param_requirements().required_count() as u32;
        let func = Box::new(move |accessor: &mut ParamsAccessor<V>| f.call(accessor))
            as Box<dyn OnceJSCallable<V>>;
        Self {
            func: FuncType::OnceFn(Some(func)),
            required_params,
        }
    }

    /// Calls the function with provided parameter accessor, returning JS result
    pub(crate) fn call(&mut self, accessor: &mut ParamsAccessor<V>) -> JSResult<V> {
        // Validate the number of arguments
        let num_args = accessor.args_len() as u32;
        if num_args < self.required_params {
            return Err(RustyJSError::InvalidParameter {
                expected: self.required_params,
                got: num_args,
            });
        }

        match &mut self.func {
            FuncType::MutFn(func) => func.call(accessor),
            FuncType::OnceFn(func) => func
                .take()
                .ok_or(RustyJSError::OnceFnCalled)?
                .once(accessor),
        }
    }

    /// Returns the number of required parameters for the function
    pub(crate) fn parameter_required_count(&self) -> u32 {
        self.required_params
    }
}

/// A wrapper for a Rust function that can be used as a constructor in JavaScript.
///
/// This struct encapsulates a `RustFunc` and provides a way to create new instances
/// of JavaScript objects using Rust functions. It is designed to be used with the
/// `JSClass` trait to define JavaScript classes with Rust constructors.
///
/// # Type Parameters
///
/// * `V`: The JavaScript value type that the constructor will work with. This type
///   must implement the `JSValueImpl` trait.
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

    fn class_setup(_class: &crate::ClassSetup<V>) -> JSResult<()> {
        Ok(())
    }
}

macro_rules! impl_js_callable_func {
    ($($t:ident),* $(,)?) => {
        // Sync function implementation - automatically chosen for functions returning direct values
        impl<V, R, Fun $(,$t)*> IntoJSCallable<V, ($($t,)*), SyncFnMut> for Fun
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
        impl<V, R, Fun, Fut $(,$t)*> IntoJSCallable<V, ($($t,)*), AsyncFnMut> for Fun
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
                Ok(Promise::from_future(ctx,future)?.into_js_value(ctx))
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

macro_rules! impl_once_js_callable_func {
    ($($t:ident),* $(,)?) => {
        // Sync function implementation - automatically chosen for functions returning direct values
        impl<V, R, Fun $(,$t)*> IntoOnceJSCallable<V, ($($t,)*), SyncFnOnce> for Fun
        where
            Fun: FnOnce ($($t),*) -> R,
            V: JSValueImpl,
            ($($t,)*): FromParams<V>,
            R: IntoJSValue<V>,
        {
            fn call(self, accessor: &mut ParamsAccessor<V>) -> JSResult<V>  {
                let params = <($($t,)*)>::from_params(accessor)?;
                #[allow(non_snake_case)]
                let ($($t,)*) = params;
                let result = (self)($($t),*);
                Ok(result.into_js_value(accessor.context()))
            }
        }

        // Async function implementation - automatically chosen for functions returning Future
        impl<V, R, Fun, Fut $(,$t)*> IntoOnceJSCallable<V, ($($t,)*), AsyncFnOnce> for Fun
        where
            Fun: FnOnce($($t),*) -> Fut,
            Fut: Future<Output = R> + 'static,
            R: IntoJSValue<V> + 'static,
            R: PromiseResolver<V>,
            V: JSValueImpl + JSObjectOps+'static,
            ($($t,)*): FromParams<V>,
        {
            fn call(self, accessor: &mut ParamsAccessor<V>) -> JSResult<V>  {
                let params = <($($t,)*)>::from_params(accessor)?;
                #[allow(non_snake_case)]
                let ($($t,)*) = params;
                let future = (self)($($t),*);
                let ctx=accessor.context();
                Ok(Promise::from_future(ctx,future)?.into_js_value(ctx))
            }
        }
    };
}

impl_once_js_callable_func!();
impl_once_js_callable_func!(A);
impl_once_js_callable_func!(A, B);
impl_once_js_callable_func!(A, B, C);
impl_once_js_callable_func!(A, B, C, D);
impl_once_js_callable_func!(A, B, C, D, E);
impl_once_js_callable_func!(A, B, C, D, E, F);
impl_once_js_callable_func!(A, B, C, D, E, F, G);
