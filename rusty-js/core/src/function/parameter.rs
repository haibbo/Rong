use crate::{FromJSValue, JSValueImpl};
use std::marker::PhantomData;
use std::ops::Deref;

/// Arguments retrieved from the JavaScript side for calling Rust functions.
pub struct ParamsAccessor<'a, V: JSValueImpl> {
    ctx: &'a V::Context,
    this: Option<V>,
    args: Vec<V>,
    is_last_param: bool,
}

impl<'a, V: JSValueImpl> ParamsAccessor<'a, V> {
    pub fn new(ctx: &'a V::Context, this: V, args: Vec<V>) -> Self {
        Self {
            ctx,
            this: Some(this),
            args,
            is_last_param: false,
        }
    }

    fn set_last_param(&mut self, is_last: bool) {
        self.is_last_param = is_last;
    }

    fn next_arg(&mut self) -> Option<V> {
        if !self.args.is_empty() {
            Some(self.args.remove(0))
        } else {
            None
        }
    }

    fn take_this(&mut self) -> Option<V> {
        self.this.take()
    }

    pub(crate) fn context(&self) -> &V::Context {
        self.ctx
    }

    // length changed since its content will be removed
    pub(crate) fn args_len(&self) -> usize {
        self.args.len()
    }
}

/// Represents the `this` context in JavaScript function calls.
///
/// # Usage
/// - Used to capture the JavaScript `this` context in Rust functions
/// - Must be the first parameter if present
/// - Does not count towards required parameter count
///
/// # Example
/// ```ignore
/// use rusty_js_core::function::parameter::This;
///
/// fn method(this: This<MyStruct>, x: i32) {
///     // Access the this context via deref
///     let my_struct: &MyStruct = &this;
/// }
/// ```
pub struct This<T>(pub T);

/// Represents an optional parameter in JavaScript function calls.
///
/// # Usage
/// - Used for parameters that may or may not be provided
/// - Wraps the parameter type in `Option<T>`
/// - Does not count towards required parameter count
/// - Can appear anywhere in the parameter list
///
/// # Example
/// ```ignore
/// use rusty_js_core::function::parameter::Optional;
///
/// fn func(x: i32, opt: Optional<String>) {
///     // Access the optional value via deref
///     if let Some(s) = &*opt {
///         println!("Optional param provided: {}", s);
///     }
/// }
/// ```
pub struct Optional<T>(pub Option<T>);

/// Represents rest parameters in JavaScript function calls.
///
/// # Usage
/// - Collects all remaining arguments into a Vec<T>
/// - Must be the last parameter if present
/// - Does not count towards required parameter count
/// - Useful for variadic functions
///
/// # Example
/// ```ignore
/// use rusty_js_core::function::parameter::Rest;
///
/// fn variadic(x: i32, rest: Rest<String>) {
///     // Access the rest parameters via deref
///     for s in &*rest {
///         println!("Rest param: {}", s);
///     }
/// }
/// ```
pub struct Rest<T>(pub Vec<T>);

impl<T> Deref for This<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Deref for Optional<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Deref for Rest<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub mod sealed {
    pub trait RegularTypeSealed {}

    impl RegularTypeSealed for i32 {}
    impl RegularTypeSealed for u32 {}
    impl RegularTypeSealed for i64 {}
    impl RegularTypeSealed for u64 {}
    impl RegularTypeSealed for f32 {}
    impl RegularTypeSealed for f64 {}
    impl RegularTypeSealed for bool {}
    impl RegularTypeSealed for String {}
    impl<T> RegularTypeSealed for Vec<T> {}
    impl<T> RegularTypeSealed for Option<T> {}
}

/// Represents parameter requirements for a function
/// - required_count: number of mandatory parameters
/// - exhaustive: if true, no extra parameters are allowed beyond the required ones
pub trait FromParams<V: JSValueImpl>: Sized {
    fn from_params(accessor: &mut ParamsAccessor<V>) -> Result<Self, String>;
    fn param_requirements() -> ParamRequirement;
}

pub struct ParamRequirement {
    required_count: usize,
    exhaustive: bool,
}

impl ParamRequirement {
    pub fn required_count(&self) -> usize {
        self.required_count
    }

    const fn single() -> Self {
        Self {
            required_count: 1,
            exhaustive: true,
        }
    }

    const fn optional() -> Self {
        Self {
            required_count: 0,
            exhaustive: false,
        }
    }

    const fn any() -> Self {
        Self {
            required_count: 0,
            exhaustive: false,
        }
    }
}

pub trait ParamKind {
    type Inner;
    fn param_requirement() -> ParamRequirement;
}

pub struct Regular<T>(PhantomData<T>);
impl<T> ParamKind for Regular<T> {
    type Inner = T;
    fn param_requirement() -> ParamRequirement {
        ParamRequirement::single()
    }
}

pub struct ThisKind<T>(PhantomData<T>);
impl<T> ParamKind for ThisKind<T> {
    type Inner = T;
    fn param_requirement() -> ParamRequirement {
        ParamRequirement::any()
    }
}

pub struct OptionalKind<T>(PhantomData<T>);
impl<T> ParamKind for OptionalKind<T> {
    type Inner = T;
    fn param_requirement() -> ParamRequirement {
        ParamRequirement::optional()
    }
}

pub struct RestKind<T>(PhantomData<T>);
impl<T> ParamKind for RestKind<T> {
    type Inner = T;
    fn param_requirement() -> ParamRequirement {
        ParamRequirement::any()
    }
}

pub trait GetParam<V: JSValueImpl> {
    type Kind: ParamKind;
    fn get_param(accessor: &mut ParamsAccessor<V>) -> Result<Self, String>
    where
        Self: Sized;
}

impl<T, V> GetParam<V> for T
where
    V: JSValueImpl,
    T: FromJSValue<V> + Sized,
    T: sealed::RegularTypeSealed,
{
    type Kind = Regular<T>;

    fn get_param(accessor: &mut ParamsAccessor<V>) -> Result<Self, String> {
        let value = accessor
            .next_arg()
            .ok_or_else(|| "Missing argument".to_string())?;
        T::from_js_value(accessor.ctx, value)
    }
}

impl<T, V> GetParam<V> for This<T>
where
    V: JSValueImpl,
    T: FromJSValue<V>,
{
    type Kind = ThisKind<T>;

    fn get_param(accessor: &mut ParamsAccessor<V>) -> Result<Self, String> {
        let value = accessor
            .take_this()
            .ok_or_else(|| "Missing this value".to_string())?;
        T::from_js_value(accessor.ctx, value).map(|v| This(v))
    }
}

impl<T, V> GetParam<V> for Optional<T>
where
    V: JSValueImpl,
    T: FromJSValue<V>,
{
    type Kind = OptionalKind<T>;

    fn get_param(accessor: &mut ParamsAccessor<V>) -> Result<Self, String> {
        match accessor.next_arg() {
            Some(v) => T::from_js_value(accessor.ctx, v).map(|t| Optional(Some(t))),
            None => Ok(Optional(None)),
        }
    }
}

impl<T, V> GetParam<V> for Rest<T>
where
    V: JSValueImpl,
    T: FromJSValue<V>,
{
    type Kind = RestKind<T>;

    fn get_param(accessor: &mut ParamsAccessor<V>) -> Result<Self, String> {
        let mut values = Vec::new();
        if accessor.is_last_param {
            while let Some(value) = accessor.next_arg() {
                values.push(T::from_js_value(accessor.ctx, value)?);
            }
        }
        Ok(Rest(values))
    }
}

macro_rules! impl_from_params {
    ($($T:ident),*) => {
        impl<V: JSValueImpl, $($T,)*> FromParams<V> for ($($T,)*)
        where
            $($T: GetParam<V>,)*
        {
            #[allow(unused_variables)]
            fn from_params(accessor: &mut ParamsAccessor<V>) -> Result<Self, String> {
                let param_count = count_idents!($($T),*);

                #[allow(unused_mut)]
                let mut current_param = 0;

                Ok(($(
                    {
                        current_param += 1;
                        accessor.set_last_param(current_param == param_count);
                        $T::get_param(accessor)?
                    },
                )*))
            }

            fn param_requirements() -> ParamRequirement {

                #[allow(unused_mut)]
                let mut req = ParamRequirement {
                    required_count: 0,
                    exhaustive: true,
                };

                $(
                    let param_req = <$T::Kind>::param_requirement();
                    req.required_count += param_req.required_count;
                    if !param_req.exhaustive {
                        req.exhaustive = false;
                    }
                )*
                req
            }
        }
    }
}

// Helper macro to count identifiers
macro_rules! count_idents {
    () => { 0 };
    ($head:ident $(,$tail:ident)*) => { 1 + count_idents!($($tail),*) };
}

// Implement for common tuple sizes
impl_from_params!();
impl_from_params!(A);
impl_from_params!(A, B);
impl_from_params!(A, B, C);
impl_from_params!(A, B, C, D);
impl_from_params!(A, B, C, D, E);
impl_from_params!(A, B, C, D, E, F);
impl_from_params!(A, B, C, D, E, F, G);
impl_from_params!(A, B, C, D, E, F, G, H);
