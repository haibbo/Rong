use crate::{
    function::JSParameterType, FromJSValue, IntoJSValue, JSContext, JSContextImpl,
    JSExceptionHandler, JSFunc, JSObject, JSObjectOps, JSResult, JSTypeOf, JSValueImpl,
    RustyJSError,
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
    ctx: JSContext<V::Context>,
}

impl<V: JSValueImpl> Clone for Promise<V> {
    fn clone(&self) -> Self {
        Self {
            obj: self.obj.clone(),
            ctx: self.ctx.clone(),
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
    fn into_js_value(self, ctx: &V::Context) -> V {
        self.obj.into_js_value(ctx)
    }
}

impl<V> FromJSValue<V> for Promise<V>
where
    V: JSTypeOf,
{
    fn from_js_value(ctx: &V::Context, value: V) -> JSResult<Self> {
        let obj = JSObject::from_js_value(ctx, value)?;
        let ctx = JSContext::from(ctx.clone());
        Ok(Self { obj, ctx })
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
        Ok((
            Promise {
                obj: promise,
                ctx: self.clone(),
            },
            resolver,
            reject,
        ))
    }
}

impl<V: JSValueImpl + 'static> Promise<V>
where
    V: JSTypeOf + JSObjectOps,
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
    /// Converts the Promise into a Future that resolves to a value of type T
    pub fn into_future<T>(self) -> PromiseFuture<V, T>
    where
        T: FromJSValue<V> + 'static,
        V::Context: 'static,
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
    V::Context: 'static,
    T: FromJSValue<V> + JSParameterType + 'static,
    V: JSObjectOps,
{
    type Output = JSResult<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        // If we haven't set up callbacks yet
        if this.state.is_none() {
            // Create initial state
            this.state = Some(Rc::new(RefCell::new(PromiseState::Pending(
                cx.waker().clone(),
            ))));

            // Get the context
            let ctx = this.promise.ctx.clone();

            // Clone state for callbacks
            let state = this.state.clone().unwrap();

            // resolved callback used to wake up future and save resolved value
            let resolve_state = state.clone();
            let resolve = JSFunc::new(&ctx, move |success: T| {
                // println!("resolve callback called");
                let mut state = resolve_state.borrow_mut();
                if let PromiseState::Pending(waker) =
                    std::mem::replace(&mut *state, PromiseState::Resolved(Ok(success)))
                {
                    waker.wake_by_ref();
                }
            });

            // rejected callback used to wake up future and save rejected value
            let reject_state = state.clone();
            let reject = JSFunc::new(&ctx, move |err: RustyJSError| {
                // println!("reject callback called");
                let mut state = reject_state.borrow_mut();
                if let PromiseState::Pending(waker) =
                    std::mem::replace(&mut *state, PromiseState::Resolved(Err(err)))
                {
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

        // Check if we have a resolved value
        if let Some(state) = &this.state {
            let mut state = state.borrow_mut();
            match &*state {
                PromiseState::Resolved(Ok(_)) => {
                    // use memory replace to avoid T depends on Clone
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
