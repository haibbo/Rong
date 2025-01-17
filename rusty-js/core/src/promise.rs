use crate::JSException;
use crate::{
    function::JSParameterType, FromJSValue, IntoJSValue, JSContext, JSContextImpl, JSFunc,
    JSObject, JSObjectOps, JSResult, JSTypeOf, JSValue, JSValueImpl, RustyJSError,
};
use std::cell::RefCell;
use std::future::Future;
use std::marker::PhantomData;
use std::ops::Deref;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

/// Type alias for the return value of `promise()` function
type PromiseResult<V> = Result<(Promise<V>, JSFunc<V>, JSFunc<V>), RustyJSError>;

/// Represents a JavaScript Promise object.
///
/// This struct wraps a JavaScript Promise and provides methods to interact with it.
pub struct Promise<V: JSValueImpl> {
    obj: JSObject<V>,
}

impl<V: JSValueImpl> Clone for Promise<V> {
    fn clone(&self) -> Self {
        Self {
            obj: self.obj.clone(),
        }
    }
}

impl<V: JSValueImpl> Deref for Promise<V> {
    type Target = JSObject<V>;
    fn deref(&self) -> &Self::Target {
        &self.obj
    }
}

impl<V> IntoJSValue<V> for Promise<V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, ctx: &JSContext<V::Context>) -> V {
        self.obj.into_js_value(ctx)
    }
}

impl<V> FromJSValue<V> for Promise<V>
where
    V: JSTypeOf,
{
    fn from_js_value(ctx: &JSContext<V::Context>, value: V) -> JSResult<Self> {
        let obj = JSObject::from_js_value(ctx, value)?;
        Ok(Self { obj })
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
        let (promise, resolver, reject) = self.as_ref().promise();
        let promise = JSObject::from_js_value(self, promise)?;
        let resolver = JSFunc::from_js_value(self, resolver)?;
        let reject = JSFunc::from_js_value(self, reject)?;
        Ok((Promise { obj: promise }, resolver, reject))
    }
}

impl<V: JSValueImpl + 'static> Promise<V>
where
    V: JSObjectOps,
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
        F: Future<Output = R> + 'static,
        R: IntoJSValue<V> + 'static,
        R: PromiseResolver<V>,
    {
        let (promise, resolve, reject) = ctx.promise()?;

        // Spawn a new async task to handle the future
        ctx.spawn_local(async move {
            let result = future.await;
            result.resolve_promise(resolve, reject);
            Ok(())
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
        self.obj.get("then").unwrap()
    }

    /// Returns the `catch` method of the Promise.
    ///
    /// This method is used to attach callbacks that will be called when the Promise is rejected.
    ///
    /// # Returns
    /// A `JSFunc<V>` representing the `catch` method of the Promise.
    pub fn catch(&self) -> JSFunc<V> {
        self.obj.get("catch").unwrap()
    }

    pub fn into_object(self) -> JSObject<V> {
        self.obj
    }
}

/// Converts a Rust future result into JavaScript Promise resolution
/// using the provided resolve/reject callbacks
pub trait PromiseResolver<V: JSValueImpl> {
    fn resolve_promise(self, resolve: JSFunc<V>, reject: JSFunc<V>);
}

// Implement for RustyJSError types
impl<V> PromiseResolver<V> for RustyJSError
where
    RustyJSError: IntoJSValue<V>,
    V: JSObjectOps,
{
    fn resolve_promise(self, _resolve: JSFunc<V>, reject: JSFunc<V>) {
        let _ = reject.call::<_, ()>((self,));
    }
}

// Implement for regular types
impl<V, T> PromiseResolver<V> for T
where
    T: IntoJSValue<V> + JSParameterType,
    V: JSObjectOps,
{
    fn resolve_promise(self, resolve: JSFunc<V>, _reject: JSFunc<V>) {
        let _ = resolve.call::<_, ()>((self,));
    }
}

pub struct PromiseFuture<V: JSValueImpl, T> {
    state: Option<Rc<RefCell<PromiseState<T>>>>,
    promise: Promise<V>,
    _marker: PhantomData<T>,
}

enum PromiseState<T> {
    Pending(Waker),
    Resolved(JSResult<T>),
}

impl<V: JSValueImpl, T> Unpin for PromiseFuture<V, T> {}

impl<V: JSValueImpl + 'static> Promise<V> {
    /// Converts the Promise into a Future that resolves to a value of type T.
    ///
    /// # Example
    /// ```
    /// let promise = ctx.eval::<Promise>(js_code)?;
    /// let future = promise.into_future::<i32>();
    /// ```
    pub fn into_future<T>(self) -> PromiseFuture<V, T>
    where
        T: FromJSValue<V> + 'static,
    {
        PromiseFuture {
            state: None,
            promise: self,
            _marker: PhantomData::<T>,
        }
    }
}

impl<V: JSValueImpl + 'static, T> Future for PromiseFuture<V, T>
where
    T: FromJSValue<V> + 'static,
    V: JSObjectOps,
{
    type Output = JSResult<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        // First poll: Initialize state and setup callbacks
        if this.state.is_none() {
            // Create initial state with waker
            let state = Rc::new(RefCell::new(PromiseState::Pending(cx.waker().clone())));
            this.state = Some(state.clone());

            let ctx = this.promise.obj.get_ctx();

            // Clone state for callbacks
            let state = this.state.clone().unwrap();

            // resolved callback used to wake up future and save resolved value
            let resolve_state = state.clone();
            let resolve = JSFunc::new(
                &ctx,
                move |ctx: &JSContext<V::Context>, value: JSValue<V>| {
                    //println!("resolve callback called");
                    let mut state = resolve_state.borrow_mut();

                    if value.is_error().is_some() || value.is_exception().is_some() {
                        let err = JSException::from_js_value(ctx, value.into_inner()).unwrap();
                        if let PromiseState::Pending(waker) = std::mem::replace(
                            &mut *state,
                            PromiseState::Resolved(Err(RustyJSError::Exception(err.into_error()))),
                        ) {
                            waker.wake_by_ref();
                        }
                    } else {
                        let success = T::from_js_value(ctx, value.into_inner()).unwrap();
                        if let PromiseState::Pending(waker) =
                            std::mem::replace(&mut *state, PromiseState::Resolved(Ok(success)))
                        {
                            waker.wake_by_ref();
                        }
                    }
                },
            );

            // rejected callback used to wake up future and save rejected value
            let reject_state = state.clone();
            let reject = JSFunc::new(&ctx, move |err: JSException<V>| {
                //println!("reject callback called");
                let mut state = reject_state.borrow_mut();
                if let PromiseState::Pending(waker) = std::mem::replace(
                    &mut *state,
                    PromiseState::Resolved(Err(RustyJSError::Exception(err.into_error()))),
                ) {
                    waker.wake_by_ref();
                }
            });

            // Register resolve handlers
            this.promise
                .then()
                .call_with_this::<_, ()>(this.promise.obj.clone(), (resolve,))?;

            // Also register catch handler for unhandled rejections
            this.promise
                .catch()
                .call_with_this::<_, ()>(this.promise.obj.clone(), (reject,))?;

            return Poll::Pending;
        }

        if let Some(state) = &this.state {
            let mut state = state.borrow_mut();

            // Then check Promise state
            match &*state {
                PromiseState::Resolved(Ok(_)) => {
                    if let PromiseState::Resolved(Ok(success)) =
                        std::mem::replace(&mut *state, PromiseState::Pending(cx.waker().clone()))
                    {
                        return Poll::Ready(Ok(success));
                    }
                }
                PromiseState::Resolved(Err(_)) => {
                    if let PromiseState::Resolved(Err(err)) =
                        std::mem::replace(&mut *state, PromiseState::Pending(cx.waker().clone()))
                    {
                        return Poll::Ready(Err(err));
                    }
                }
                PromiseState::Pending(_) => {
                    // Update the waker
                    *state = PromiseState::Pending(cx.waker().clone());
                }
            }
        }

        Poll::Pending
    }
}
