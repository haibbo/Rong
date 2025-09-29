use crate::{
    IntoJSValue, JSClass, JSObjectOps, JSResult, JSValueImpl, Promise, PromiseResolver, RongJSError,
};
use std::cell::RefCell;
use std::future::Future;

mod parameter;
pub use parameter::{FromParams, JSParameterType, Optional, ParamsAccessor, Rest, This, ThisMut};

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
/// // Sync function
/// RustFunc::new(|x: i32| x + 1);
///
/// // Async function
/// RustFunc::new(async |x: i32| {
///     // async operation
///     x + 1
/// });
/// ```
pub(crate) struct RustFunc<V: JSValueImpl> {
    func: JSCallable<V>,
    required_params: u32,
}

/// Type alias for FnMut closure type
type FnMutClosure<V> = dyn FnMut(&mut ParamsAccessor<V>) -> JSResult<V>;

/// Type alias for FnOnce closure type
type FnOnceClosure<V> = dyn FnOnce(&mut ParamsAccessor<V>) -> JSResult<V>;

// since Fn depends on FnMut, FuMut discriminant is for both Fn and FnMut
pub enum JSCallable<V: JSValueImpl> {
    FnMut(RefCell<Box<FnMutClosure<V>>>),
    FnOnce(RefCell<Option<Box<FnOnceClosure<V>>>>),
}

/// Trait for converting Rust functions into JavaScript callable functions.
/// Type parameters:
/// - V: The JavaScript value type
/// - P: Parameter types tuple
/// - K: marker type to avoid rustc complain confiction implementation
pub trait IntoJSCallable<V: JSValueImpl, P, K> {
    fn into_js_callable(self) -> JSCallable<V>;
}

/// same as IntoJSCallable, but it's for once callable function
pub trait IntoOnceJSCallable<V: JSValueImpl, P, K> {
    fn into_js_callable(self) -> JSCallable<V>;
}

/// Marker type to let rustc happy to avoid it complain confliction implementation
/// since Fn depends on FnMut, and FnMut depends on FnOnce, when P is the same, rustc
/// consider confliction implementation for Fn,FnMut etc.
pub struct KFnMut;
pub struct KFnOnce;
pub struct KAsyncFnMut;
pub struct KAsyncFnOnce;

impl<V: JSValueImpl> RustFunc<V> {
    /// Creates a new RustFunc instance for wrapping a multi-callable Rust closure/function
    pub(crate) fn new<F, P, K>(f: F) -> Self
    where
        F: IntoJSCallable<V, P, K>,
        P: FromParams<V>,
    {
        let required_params = P::param_requirements().required_count() as u32;
        Self {
            func: f.into_js_callable(),
            required_params,
        }
    }

    pub(crate) fn new_once<F, P, K>(f: F) -> Self
    where
        F: IntoOnceJSCallable<V, P, K>,
        P: FromParams<V>,
    {
        let required_params = P::param_requirements().required_count() as u32;
        Self {
            func: f.into_js_callable(),
            required_params,
        }
    }

    /// Calls the function with provided parameter accessor, returning JS result
    pub(crate) fn call(&mut self, accessor: &mut ParamsAccessor<V>) -> JSResult<V> {
        // Validate the number of arguments
        let num_args = accessor.args_len() as u32;
        if num_args < self.required_params {
            return Err(RongJSError::InvalidParameter {
                expected: self.required_params,
                got: num_args,
            });
        }

        match &self.func {
            JSCallable::FnMut(f) => f.borrow_mut()(accessor),
            JSCallable::FnOnce(f) => f.take().ok_or(RongJSError::OnceFnCalled)?(accessor),
        }
    }

    /// Returns the number of required parameters for the function
    pub(crate) fn parameter_required_count(&self) -> u32 {
        self.required_params
    }
}

/// A wrapper for a Rust function that can be used to handles both class construction (`new T()`)
/// and implicit constructor invocation(`T()`) behaviors in JavaScript.
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
        F: IntoJSCallable<V, P, K>,
        P: FromParams<V>,
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
        // Sync FnMut function/closure implementation
        impl<V, R, Fun $(,$t)*> IntoJSCallable<V, ($($t,)*), KFnMut> for Fun
        where
            Fun: FnMut($($t),*) -> R + 'static,
            V: JSValueImpl,
            ($($t,)*): FromParams<V>,
            R: IntoJSValue<V>
        {
            fn into_js_callable(self) -> JSCallable<V> {
                let mut f = self;
                let closure = move |accessor: &mut ParamsAccessor<V>| {
                    let params = <($($t,)*)>::from_params(accessor)?;
                    #[allow(non_snake_case)]
                    let ($($t,)*) = params;
                    let result = f($($t),*);
                    Ok(result.into_js_value(accessor.context()))
                };
                JSCallable::FnMut(RefCell::new(Box::new(closure)))
            }
        }


        // Async FnMut implementation
        impl<V, R, Fut, Fun $(,$t)*> IntoJSCallable<V, ($($t,)*), KAsyncFnMut> for Fun
        where
            Fun: FnMut($($t),*) -> Fut + 'static,
            Fut: Future<Output=R> +'static,
            V: JSValueImpl + JSObjectOps + 'static,
            ($($t,)*): FromParams<V> ,
            R: IntoJSValue<V> + PromiseResolver<V> + 'static,
        {
            fn into_js_callable(self) -> JSCallable<V> {
                let mut f = self;
                let closure = move |accessor: &mut ParamsAccessor<V>| {
                    let params = <($($t,)*)>::from_params(accessor)?;
                    #[allow(non_snake_case)]
                    let ($($t,)*) = params;
                    let future = f($($t),*);
                    let ctx = accessor.context();
                    Ok(Promise::from_future(ctx, future)?.into_js_value(ctx))
                };
                JSCallable::FnMut(RefCell::new(Box::new(closure)))
            }
        }
    };
 }

macro_rules! impl_js_oncecallable_func {
    ($($t:ident),* $(,)?) => {
        // FnOnce function/closure implementation
        impl<V, R, Fun $(,$t)*> IntoOnceJSCallable<V, ($($t,)*), KFnOnce> for Fun
        where
            Fun: FnOnce($($t),*) -> R + 'static,
            V: JSValueImpl,
            ($($t,)*): FromParams<V>,
            R: IntoJSValue<V>
        {
            fn into_js_callable(self) -> JSCallable<V> {
                let f = self;
                let closure = move |accessor: &mut ParamsAccessor<V>| {
                    let params = <($($t,)*)>::from_params(accessor)?;
                    #[allow(non_snake_case)]
                    let ($($t,)*) = params;
                    let result = f($($t),*);
                    Ok(result.into_js_value(accessor.context()))
                };
                JSCallable::FnOnce(RefCell::new(Some(Box::new(closure))))
            }
        }

        // Async Fn implementation
        impl<V, R,Fut, Fun $(,$t)*> IntoOnceJSCallable<V, ($($t,)*), KAsyncFnOnce> for Fun
        where
            Fun: FnOnce($($t),*) -> Fut + 'static,
            Fut: Future<Output=R> +'static,
            V: JSValueImpl + JSObjectOps + 'static,
            ($($t,)*): FromParams<V>,
            R: IntoJSValue<V> + PromiseResolver<V> + 'static,
        {
            fn into_js_callable(self) -> JSCallable<V> {
                let f = self;
                let closure = move |accessor: &mut ParamsAccessor<V>| {
                    let params = <($($t,)*)>::from_params(accessor)?;
                    #[allow(non_snake_case)]
                    let ($($t,)*) = params;
                    let future = f($($t),*);
                    let ctx = accessor.context();
                    Ok(Promise::from_future(ctx, future)?.into_js_value(ctx))
                };
                JSCallable::FnOnce(RefCell::new(Some(Box::new(closure))))
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

impl_js_oncecallable_func!();
impl_js_oncecallable_func!(P1);
impl_js_oncecallable_func!(P1, P2);
impl_js_oncecallable_func!(P1, P2, P3);
impl_js_oncecallable_func!(P1, P2, P3, P4);
impl_js_oncecallable_func!(P1, P2, P3, P4, P5);
impl_js_oncecallable_func!(P1, P2, P3, P4, P5, P6);
impl_js_oncecallable_func!(P1, P2, P3, P4, P5, P6, P7);
impl_js_oncecallable_func!(P1, P2, P3, P4, P5, P6, P7, P8);
