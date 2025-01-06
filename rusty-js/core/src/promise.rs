use crate::{
    FromJSValue, IntoJSValue, JSContext, JSContextImpl, JSExceptionHandler, JSFunc, JSObject,
    JSObjectOps, JSResult, JSTypeOf, JSValueImpl, RustyJSError,
};
use std::future::Future;

/// Type alias for the return value of `promise()` function
type PromiseResult<V> = Result<(Promise<V>, JSFunc<V>, JSFunc<V>), RustyJSError>;

/// Represents a JavaScript Promise object.
///
/// This struct wraps a JavaScript Promise and provides methods to interact with it.
pub struct Promise<V: JSValueImpl>(JSObject<V>);

impl<V: JSValueImpl> Clone for Promise<V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<V> IntoJSValue<V> for Promise<V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, ctx: &V::Context) -> V {
        self.0.into_js_value(ctx)
    }
}

impl<C: JSContextImpl> JSContext<C> {
    /// Creates a new JavaScript Promise and returns the Promise along with its resolve and reject functions.
    ///
    /// # Returns
    /// A `Result` containing a tuple of:
    /// - The `Promise` object
    /// - The resolve function (`JSFunc<V>`)
    /// - The reject function (`JSFunc<V>`)
    ///
    /// # Errors
    /// Returns a `RustyJSError` if the Promise creation fails.
    pub fn promise(&self) -> PromiseResult<C::Value>
    where
        C::Value: JSTypeOf,
    {
        let (promise, resolver, reject) = self.inner.promise();
        let promise = JSObject::from_js_value(&self.inner, promise)?;
        let resolver = JSFunc::from_js_value(&self.inner, resolver)?;
        let reject = JSFunc::from_js_value(&self.inner, reject)?;

        Ok((Promise(promise), resolver, reject))
    }
}

impl<V: JSValueImpl + 'static> Promise<V>
where
    V: JSTypeOf,
    V: JSObjectOps,
    V::Context: 'static,
{
    /// Creates a new JavaScript Promise using the provided context.
    ///
    /// # Arguments
    /// * `ctx` - The JavaScript context in which to create the Promise.
    ///
    /// # Returns
    /// A `Result` containing a tuple of:
    /// - The `Promise` object
    /// - The resolve function (`JSFunc<V>`)
    /// - The reject function (`JSFunc<V>`)
    ///
    /// # Errors
    /// Returns a `RustyJSError` if the Promise creation fails.
    pub fn new(ctx: &JSContext<V::Context>) -> PromiseResult<V> {
        ctx.promise()
    }

    /// Converts a Rust Future into a JavaScript Promise.
    ///
    /// # Returns
    /// A `Result` containing the Promise object
    ///
    /// # Errors
    /// Returns a `RustyJSError` if the Promise creation fails.
    pub fn from_future<F, R>(ctx: &JSContext<V::Context>, future: F) -> JSResult<Promise<V>>
    where
        F: Future<Output = JSResult<R>> + 'static,
        R: IntoJSValue<V> + 'static,
        V::Context: JSExceptionHandler,
    {
        let (promise, resolve, reject) = ctx.promise()?;

        // Clone context for the async task
        let task_ctx = ctx.clone();

        // Spawn a new async task to handle the future
        tokio::task::spawn_local(async move {
            match future.await {
                Ok(value) => {
                    let _ = resolve.call::<_, ()>((value,));
                }
                Err(err) => {
                    let js_error = err.into_js_error(&task_ctx);
                    let _ = reject.call::<_, ()>((js_error,));
                }
            }
        });

        Ok(promise)
    }

    /// Returns the `then` method of the Promise.
    ///
    /// This method is used to attach callbacks that will be called when the Promise is resolved.
    ///
    /// # Returns
    /// A `JSFunc<V>` representing the `then` method of the Promise.
    pub fn then(&self) -> JSFunc<V> {
        self.0.get("then").unwrap()
    }

    /// Returns the `catch` method of the Promise.
    ///
    /// This method is used to attach callbacks that will be called when the Promise is rejected.
    ///
    /// # Returns
    /// A `JSFunc<V>` representing the `catch` method of the Promise.
    pub fn catch(&self) -> JSFunc<V> {
        self.0.get("catch").unwrap()
    }

    pub fn into_object(self) -> JSObject<V> {
        self.0
    }
}
