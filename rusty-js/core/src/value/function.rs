use crate::function::{FromParams, IntoJSCallable, RustFunc};
use crate::{
    Class, FromJSValue, IntoJSValue, JSContext, JSContextImpl, JSObject, JSObjectOps, JSTypeOf,
    JSValue, JSValueImpl, RustyJSError,
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

impl<V> IntoJSValue<V> for JSFunc<V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, ctx: &V::Context) -> V {
        self.0.into_js_value(ctx)
    }
}

impl<V> FromJSValue<V> for JSFunc<V>
where
    V: JSTypeOf,
{
    fn from_js_value(ctx: &V::Context, value: V) -> Result<Self, RustyJSError> {
        if value.is_function() {
            JSObject::from_js_value(ctx, value).map(|obj| Self(obj))
        } else {
            Err(RustyJSError::NotJSFunc)
        }
    }
}

impl<V: JSObjectOps> JSFunc<V> {
    /// Create a new JS function from a Rust function or closure
    pub fn new<C, F, P>(ctx: &JSContext<C>, f: F) -> Self
    where
        C: JSContextImpl<Value = V>,
        F: IntoJSCallable<V, P> + 'static,
        P: FromParams<V>,
        V: 'static,
    {
        ctx.register_function(f)
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
    /// let result: i32 = func.call(42)?;
    ///
    /// // Call with multiple arguments
    /// let result: String = func.call((1, "two", 3.0))?;
    /// ```
    pub fn call<Args, R>(&self, args: Args) -> Result<R, RustyJSError>
    where
        Args: IntoJSArgs<V>,
        R: FromJSValue<V>,
        V: JSObjectOps,
    {
        let ctx = self.as_ctx();
        let argv = args.into_js_args(ctx);
        let r = ctx.call(self.as_inner(), None, argv);
        let result = JSValue::from_js_value(ctx, r)?;

        result.is_exception().map_or_else(
            || R::from_js_value(ctx, result.into_inner()),
            |exception| Err(RustyJSError::Exception(exception.into_error())),
        )
    }

    /// set name of JS Function
    pub fn name(self, name: &str) -> Self {
        self.0.set("name", name);
        self
    }

    pub(crate) fn into_inner(self) -> V {
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
        P: FromParams<C::Value>,
    {
        let func = RustFunc::new(f);
        let length = func.parameter_required_count();
        let value = Class::get::<RustFunc<C::Value>>(&self.inner)
            .map(|class| class.instance::<RustFunc<C::Value>>(func));
        let obj = JSObject::from_js_value(&self.inner, value.unwrap()).unwrap();
        obj.set("length", length);
        JSFunc(obj)
    }
}
