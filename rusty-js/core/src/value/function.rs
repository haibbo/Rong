use crate::function::{FromParams, IntoJSCallable, JSParameterType, RustFunc};
use crate::{
    Class, FromJSValue, IntoJSValue, JSContext, JSContextImpl, JSException, JSObject, JSObjectOps,
    JSResult, JSTypeOf, JSValueImpl, RustyJSError,
};
use std::ops::Deref;

mod args;
pub use args::IntoJSArgs;

pub struct JSFunc<V: JSValueImpl>(JSObject<V>);

impl<V: JSValueImpl> Deref for JSFunc<V> {
    type Target = JSObject<V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V: JSValueImpl> Clone for JSFunc<V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<V> IntoJSValue<V> for JSFunc<V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, ctx: &JSContext<V::Context>) -> V {
        self.0.into_js_value(ctx)
    }
}

impl<V> FromJSValue<V> for JSFunc<V>
where
    V: JSTypeOf,
{
    fn from_js_value(ctx: &JSContext<V::Context>, value: V) -> JSResult<Self> {
        if value.is_function() {
            JSObject::from_js_value(ctx, value).map(|obj| Self(obj))
        } else {
            Err(RustyJSError::NotJSFunc)
        }
    }
}

impl<V: JSObjectOps> JSFunc<V> {
    /// Create a new JS function from a Rust function or closure
    pub fn new<C, F, P, K: 'static>(ctx: &JSContext<C>, f: F) -> Self
    where
        C: JSContextImpl<Value = V>,
        F: IntoJSCallable<V, P, K> + 'static,
        P: FromParams<V>,
        V: 'static,
    {
        ctx.register_function(f)
    }

    #[inline]
    fn call_internal<Args, R>(&self, this: Option<V>, args: Args) -> JSResult<R>
    where
        Args: IntoJSArgs<V>,
        R: FromJSValue<V>,
        V: JSObjectOps,
    {
        let ctx = &self.get_ctx();
        let argv = args.into_js_args(ctx);
        let result = ctx.as_ref().call(self.as_value(), this, argv);

        // let result = JSValue::from_js_value(ctx, r)?;

        if result.is_exception() {
            let exception = JSException::from_js_value(ctx, result)?;
            Err(RustyJSError::Exception(exception.into_error()))
        } else {
            R::from_js_value(ctx, result)
        }
    }

    /// Calls the JavaScript function with the given arguments.
    ///
    /// # Arguments
    /// * `args` - Arguments to pass to the function. Can be:
    ///   - A single value implementing `IntoJSArg`
    ///   - A tuple of values implementing `IntoJSArg` (up to 12 arguments)
    ///
    /// # Returns
    /// Returns `Ok(R)` if the call succeeds, where `R` is the return type.
    /// Returns `Err(RustyJSError)` if the call fails or throws an exception.
    ///
    /// # Examples
    /// ```rust
    /// // Call with single argument
    /// let result: i32 = func.call((42,))?;
    ///
    /// // Call with multiple arguments
    /// let result: String = func.call((1, "two", 3.0))?;
    ///
    /// // Alternatively, use the call! macro for more ergonomic syntax:
    /// let result: i32 = call!(func, 42)?;
    /// let result: String = call!(func, 1, "two", 3.0)?;
    /// ```
    pub fn call<Args, R>(&self, args: Args) -> JSResult<R>
    where
        Args: IntoJSArgs<V>,
        R: FromJSValue<V>,
        V: JSObjectOps,
    {
        self.call_internal(None, args)
    }

    /// same as `call`, but with JS this object
    pub fn call_with_this<Args, R>(&self, this: JSObject<V>, args: Args) -> JSResult<R>
    where
        Args: IntoJSArgs<V>,
        R: FromJSValue<V>,
        V: JSObjectOps,
    {
        self.call_internal(Some(this.into_js_value(&self.get_ctx())), args)
    }

    /// set name of JS Function
    pub fn name(self, name: &str) -> Self {
        self.0.set("name", name);
        self
    }

    pub(crate) fn into_inner(self) -> V {
        self.0.into_value()
    }
}

/// Macro for more ergonomic function calls
/// Examples:
/// ```ignore
/// call!(func);  // for no args
/// call!(func, arg1);  // for single arg
/// call!(func, arg1, arg2);  // for multiple args
/// ```
#[macro_export]
macro_rules! call {
    ($func:expr) => {
        $func.call(())
    };
    ($func:expr, $arg:expr) => {
        $func.call(($arg,))
    };
    ($func:expr, $($arg:expr),+ $(,)?) => {
        $func.call(($($arg,)+))
    };
}

impl<C: JSContextImpl> JSContext<C>
where
    C::Value: JSObjectOps + 'static,
{
    pub fn register_function<F, P, K>(&self, f: F) -> JSFunc<C::Value>
    where
        F: IntoJSCallable<C::Value, P, K> + 'static,
        P: FromParams<C::Value>,
        K: 'static,
    {
        let func = RustFunc::new(f);
        let length = func.parameter_required_count();
        let value = Class::get::<RustFunc<C::Value>>(self)
            .map(|class| class.instance::<RustFunc<C::Value>>(func))
            .expect("Not Found RustFunc Class");
        let obj = JSObject::from_js_value(self, value).unwrap();
        obj.set("length", length);
        JSFunc(obj)
    }
}

// blanket implementing.
// Type JSFunc can be as parameter of JS callback of rust function
impl<V: JSValueImpl> JSParameterType for JSFunc<V> {}
