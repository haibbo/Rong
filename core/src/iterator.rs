use crate::rong::spawn;
use crate::{IntoJSValue, JSContext, JSFunc, JSObject, JSObjectOps, JSResult, JSSymbol, JSValue};
use futures::{Stream, StreamExt};
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use tokio::sync::Mutex;

/// Core JavaScript iterator that wraps Rust iterators
///
/// This provides a unified way to create JavaScript iterators from Rust iterators.
/// It follows the JavaScript iteration protocol and can be used with `for...of` loops.
///
/// https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Iteration_protocols#the_iterable_protocol
/// In order to be iterable, an object must implement the [Symbol.iterator]() method,
/// meaning that the object (or one of the objects up its prototype chain) must have
/// a property with a [Symbol.iterator] key which is available via constant Symbol.iterator.
pub struct JSIterator<V, T>
where
    V: JSObjectOps + 'static,
    T: IntoJSValue<V> + 'static,
{
    /// The underlying Rust iterator wrapped in Rc<RefCell<>> for shared mutable access
    inner: Rc<RefCell<Box<dyn Iterator<Item = T> + 'static>>>,
    /// Cached result object to avoid creating new objects on each iteration
    result: JSObject<V>,
}

impl<V, T> JSIterator<V, T>
where
    V: JSObjectOps + 'static,
    T: IntoJSValue<V> + 'static,
{
    /// Create a new JSIterator from any type that implements IntoIterator
    pub fn from<I>(iterable: I, ctx: &JSContext<V::Context>) -> Self
    where
        I: IntoIterator<Item = T> + 'static,
        I::IntoIter: 'static,
    {
        Self {
            inner: Rc::new(RefCell::new(Box::new(iterable.into_iter()))),
            result: JSObject::new(ctx),
        }
    }

    /// The next() method of the iterator protocol
    pub fn next(&self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>> {
        let result = self.result.clone();
        let mut iter = self.inner.borrow_mut();

        match iter.next() {
            Some(item) => {
                result.set("done", false)?;
                let js_value = item.into_js_value(ctx);
                let js_value = JSValue::from_raw(ctx, js_value);
                result.set("value", js_value)?;
            }
            None => {
                result.set("done", true)?;
                result.set("value", JSValue::undefined(ctx))?;
            }
        }

        Ok(result)
    }

    /// Convert this iterator to a JavaScript iterable object
    pub fn to_js_iterable(&self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>> {
        let iterable = JSObject::new(ctx);

        // Create a simple object that has the iterator protocol
        let iterator_obj = JSObject::new(ctx);

        // Add the next method to the iterator object
        let iterator_instance = self.clone();
        let next_fn = JSFunc::new(ctx, move |ctx: JSContext<V::Context>| -> JSObject<V> {
            iterator_instance.next(&ctx).unwrap_or_else(|_| {
                let result = JSObject::new(&ctx);
                result.set("done", true).ok();
                result.set("value", JSValue::undefined(&ctx)).ok();
                result
            })
        })?;

        iterator_obj.set("next", next_fn)?;

        // Create Symbol.iterator function that returns the iterator object
        let iter_obj_clone = iterator_obj.clone();
        let iterator_fn = JSFunc::new(ctx, move || iter_obj_clone.clone())?;

        // Get Symbol.iterator and set it on the iterable object
        let symbol = ctx
            .global()
            .get::<_, JSObject<V>>("Symbol")?
            .get::<_, JSSymbol<V>>("iterator")?;

        iterable.set(symbol, iterator_fn)?;
        Ok(iterable)
    }
}

impl<V, T> Clone for JSIterator<V, T>
where
    V: JSObjectOps + 'static,
    T: IntoJSValue<V> + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            result: self.result.clone(),
        }
    }
}

/// Core JavaScript async iterator that wraps Rust streams
pub struct JSAsyncIterator<V, T>
where
    V: JSObjectOps + 'static,
    T: IntoJSValue<V> + Send + 'static,
{
    /// The underlying Rust stream wrapped in Rc<Mutex<>> for async shared access
    stream: Rc<Mutex<Pin<Box<dyn Stream<Item = T> + Send + 'static>>>>,
    /// Cached result object to avoid creating new objects on each iteration
    result: JSObject<V>,
}

impl<V, T> JSAsyncIterator<V, T>
where
    V: JSObjectOps + 'static,
    T: IntoJSValue<V> + Send + 'static,
{
    /// Create a new JSAsyncIterator from any type that implements Stream
    pub fn from<S>(stream: S, ctx: &JSContext<V::Context>) -> Self
    where
        S: Stream<Item = T> + Send + 'static,
    {
        Self {
            stream: Rc::new(Mutex::new(Box::pin(stream))),
            result: JSObject::new(ctx),
        }
    }

    /// The next() method of the async iterator protocol
    pub async fn next(&self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>> {
        let result = self.result.clone();
        let mut stream = self.stream.lock().await;

        match stream.next().await {
            Some(item) => {
                result.set("done", false)?;
                let js_value = item.into_js_value(ctx);
                let js_value = JSValue::from_raw(ctx, js_value);
                result.set("value", js_value)?;
            }
            None => {
                result.set("done", true)?;
                result.set("value", JSValue::undefined(ctx))?;
            }
        }

        Ok(result)
    }

    /// Convert this async iterator to a JavaScript async iterable object
    pub fn to_js_async_iterable(&self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>> {
        let iterable = JSObject::new(ctx);

        // Create a simple object that has the async iterator protocol
        let iterator_obj = JSObject::new(ctx);

        // Add the next method to the async iterator object
        let iterator_instance = self.clone();
        let next_fn = JSFunc::new(ctx, move |ctx: JSContext<V::Context>| -> JSObject<V> {
            // For async iterators, we need to return a Promise that resolves to the result
            match ctx.promise() {
                Ok((promise, resolve, _reject)) => {
                    let iterator_clone = iterator_instance.clone();

                    // Spawn a task to handle the async operation
                    spawn(async move {
                        match iterator_clone.next(&ctx).await {
                            Ok(result) => {
                                let _ = resolve.call::<_, ()>(None, (result,));
                            }
                            Err(_) => {
                                let result = JSObject::new(&ctx);
                                result.set("done", true).ok();
                                result.set("value", JSValue::undefined(&ctx)).ok();
                                let _ = resolve.call::<_, ()>(None, (result,));
                            }
                        }
                    });

                    promise.into_object()
                }
                Err(_) => {
                    // Fallback: return a resolved promise with done: true
                    let result = JSObject::new(&ctx);
                    result.set("done", true).ok();
                    result.set("value", JSValue::undefined(&ctx)).ok();
                    result
                }
            }
        })?;

        iterator_obj.set("next", next_fn)?;

        // Create Symbol.asyncIterator function that returns the iterator object
        let iter_obj_clone = iterator_obj.clone();
        let iterator_fn = JSFunc::new(ctx, move |_ctx: JSContext<V::Context>| -> JSObject<V> {
            iter_obj_clone.clone()
        })?;

        // Get Symbol.asyncIterator and set it on the iterable object
        let symbol = ctx
            .global()
            .get::<_, JSObject<V>>("Symbol")?
            .get::<_, JSSymbol<V>>("asyncIterator")?;

        iterable.set(symbol, iterator_fn)?;
        Ok(iterable)
    }
}

impl<V, T> Clone for JSAsyncIterator<V, T>
where
    V: JSObjectOps + 'static,
    T: IntoJSValue<V> + Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            stream: self.stream.clone(),
            result: self.result.clone(),
        }
    }
}

/// Extension trait for converting iterables to JavaScript iterators
pub trait IntoJSIteratorExt<V, T>
where
    V: JSObjectOps + 'static,
{
    fn to_js_iter(self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>>;
}

/// Auto-implementation for all IntoIterator types
impl<V, T, I> IntoJSIteratorExt<V, T> for I
where
    V: JSObjectOps + 'static,
    T: IntoJSValue<V> + 'static,
    I: IntoIterator<Item = T> + 'static,
    I::IntoIter: 'static,
{
    fn to_js_iter(self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>> {
        let js_iter = JSIterator::from(self, ctx);
        js_iter.to_js_iterable(ctx)
    }
}

/// Extension trait for converting streams to JavaScript async iterators
pub trait IntoJSAsyncIteratorExt<V, T>
where
    V: JSObjectOps + 'static,
{
    fn to_js_async_iter(self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>>;
}

/// Auto-implementation for all Stream types
impl<V, T, S> IntoJSAsyncIteratorExt<V, T> for S
where
    V: JSObjectOps + 'static,
    T: IntoJSValue<V> + Send + 'static,
    S: Stream<Item = T> + Send + 'static,
{
    fn to_js_async_iter(self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>> {
        let js_iter = JSAsyncIterator::from(self, ctx);
        js_iter.to_js_async_iterable(ctx)
    }
}
