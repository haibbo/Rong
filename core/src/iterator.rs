use crate::{
    function::{Constructor, ThisMut},
    Class, IntoJSValue, JSClass, JSContext, JSExceptionHandler, JSFunc, JSObject, JSObjectOps,
    JSResult, JSSymbol, JSValue,
};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::rc::Rc;
use tokio::sync::Mutex;

/// Converts an iterator into a JavaScript iterable object
///
/// This trait provides a method to convert any Rust iterator into a JavaScript iterable
/// that follows the JavaScript iteration protocol. The resulting object can be used
/// with `for...of` loops and other JavaScript constructs that expect iterables.
///
/// # Example
/// ```rust
/// use crate::ToJSIterator;
///
/// let vec = vec![1, 2, 3];
/// let iterable = vec.to_js_iter(ctx)?;
/// ```
///
/// This will create a JavaScript iterable that yields the values 1, 2, and 3
pub trait ToJSIterator<T>
where
    Self: IntoIterator<Item = T> + 'static + Sized + Clone,
{
    fn to_js_iter<V>(&self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>>
    where
        T: IntoJSValue<V>,
        V: JSObjectOps + 'static,
        V::Context: JSExceptionHandler,
    {
        let iterable = JSObject::new(ctx);
        let target = self.clone();

        // MDN:
        // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Iteration_protocols#the_iterable_protocol
        // In order to be iterable, an object must implement the [Symbol.iterator]()
        // method, meaning that the object (or one of the objects up its prototype
        // chain) must have a property with a [Symbol.iterator] key which is available
        // via constant Symbol.iterator.
        let iterator = JSFunc::new(ctx, move |ctx: JSContext<V::Context>| {
            let obj = JSObject::new(&ctx);
            let mut iter = target.clone().into_iter();

            // store result of next to avoid create object on each next
            let result = JSObject::new(&ctx);

            let next = JSFunc::new(&ctx, move |ctx: JSContext<V::Context>| {
                let result = result.clone();
                match iter.next() {
                    Some(i) => {
                        let _ = result.set("done", false);
                        let value = i.into_js_value(&ctx);
                        let value = JSValue::from_raw(&ctx, value);
                        result.set("value", value)?;
                    }
                    None => {
                        result.set("done", true)?;
                    }
                }
                Ok(result)
            })?;

            obj.set("next", next)?;
            Ok(obj)
        })?;

        let constant = ctx
            .global()
            .get::<_, JSObject<V>>("Symbol")?
            .get::<_, JSSymbol<V>>("iterator")?;
        iterable.set(constant, iterator)?;
        Ok(iterable)
    }
}

/// Auto-implementation of ToJSIterator for all types that satisfy the trait bounds
impl<T, I> ToJSIterator<T> for I
where
    I: IntoIterator<Item = T>,
    I: Clone,
    I: 'static,
{
}

/// Converts an async iterator (Stream) into a JavaScript async iterable object
///
/// This trait provides a method to convert any Rust async iterator (Stream) into
/// a JavaScript async iterable that follows the JavaScript async iteration protocol.
/// The resulting object can be used with `for await...of` loops and other JavaScript
/// constructs that expect async iterables.
///
/// # Example
/// ```rust
/// use crate::ToJSAsyncIterator;
/// use futures::stream;
///
/// let stream = stream::iter(vec![1, 2, 3]);
/// let async_iterable = stream.to_js_async_iter(ctx)?;
///
pub trait ToJSAsyncIterator<T>
where
    Self: Stream<Item = T> + 'static + Sized + Clone,
{
    fn to_js_async_iter<V>(&self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>>
    where
        T: IntoJSValue<V> + 'static,
        V: JSObjectOps + 'static,
        V::Context: JSExceptionHandler,
        Self: Send,
    {
        ctx.register_class::<JSAsyncIterator<V, T>>()?;

        let iterable = JSObject::new(ctx);
        let target = self.clone();

        let async_iterator = JSFunc::new(ctx, move |ctx: JSContext<V::Context>| {
            let stream: Rc<Mutex<Pin<Box<dyn Stream<Item = T> + Send>>>> =
                Rc::new(Mutex::new(Box::pin(target.clone())));
            let obj = Class::get::<JSAsyncIterator<V, T>>(&ctx)?
                .instance(JSAsyncIterator::new(stream.clone(), &ctx));

            let next = JSFunc::new(
                &ctx,
                async move |this: ThisMut<JSAsyncIterator<V, T>>, ctx: JSContext<V::Context>| {
                    let result = this.result.clone();

                    let mut stream = this.stream.lock().await;
                    match stream.next().await {
                        Some(value) => {
                            result.set("done", false)?;
                            let js_value = value.into_js_value(&ctx);
                            let js_value = JSValue::from_raw(&ctx, js_value);
                            result.set("value", js_value)?;
                        }
                        None => {
                            result.set("done", true)?;
                            result.set("value", JSValue::undefined(&ctx))?;
                        }
                    }

                    Ok(result)
                },
            )?;
            obj.set("next", next)?;
            Ok(obj)
        })?;

        // constant of Symbol.asyncIterator
        let symbol = ctx.global().get::<_, JSObject<V>>("Symbol")?;
        let symbol_iterator = symbol.get::<_, JSSymbol<V>>("asyncIterator")?;

        iterable.set(symbol_iterator, async_iterator)?;
        Ok(iterable)
    }
}

/// Auto-implementation of ToJSAsyncIterator for all types that satisfy the trait bounds
impl<T, S> ToJSAsyncIterator<T> for S where S: Stream<Item = T> + Clone + 'static {}

pub struct JSAsyncIterator<V, T>
where
    V: JSObjectOps,
{
    stream: Rc<Mutex<Pin<Box<dyn Stream<Item = T> + Send + 'static>>>>,
    result: JSObject<V>,
}

impl<V, T> JSAsyncIterator<V, T>
where
    V: JSObjectOps,
{
    fn new(
        stream: Rc<Mutex<Pin<Box<dyn Stream<Item = T> + Send + 'static>>>>,
        ctx: &JSContext<V::Context>,
    ) -> Self {
        Self {
            stream,
            result: JSObject::new(ctx),
        }
    }
}

impl<V, T> JSClass<V> for JSAsyncIterator<V, T>
where
    V: JSObjectOps + 'static,
    T: 'static,
{
    const NAME: &'static str = "AsyncIterator";

    // empty constructor
    fn data_constructor() -> Constructor<V> {
        Constructor::new(|| {})
    }

    fn class_setup(_class: &crate::ClassSetup<V>) -> JSResult<()> {
        // We don't need to add a next method here
        Ok(())
    }
}

/// Converts an async iterator (Stream) into a JavaScript async iterable object by consuming self
///
/// This trait provides a method to convert any Rust async iterator (Stream) into
/// a JavaScript async iterable that follows the JavaScript async iteration protocol.
/// The resulting object can be used with `for await...of` loops and other JavaScript
/// constructs that expect async iterables.
///
/// # Example
/// ```rust
/// use crate::IntoJSAsyncIterator;
/// use futures::stream;
///
/// let stream = stream::iter(vec![1, 2, 3]);
/// let async_iterable = stream.into_js_async_iter(ctx)?;
/// ```
pub trait IntoJSAsyncIterator<T>
where
    Self: Stream<Item = T> + 'static + Sized,
{
    fn into_js_async_iter<V>(self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>>
    where
        T: IntoJSValue<V> + 'static,
        V: JSObjectOps + 'static,
        V::Context: JSExceptionHandler,
        Self: Send,
    {
        ctx.register_class::<JSAsyncIterator<V, T>>()?;

        let iterable = JSObject::new(ctx);

        // Wrap self in an Rc to avoid moving it into the closure
        let stream_rc = Rc::new(Mutex::new(
            Box::pin(self) as Pin<Box<dyn Stream<Item = T> + Send>>
        ));

        let async_iterator = JSFunc::new(ctx, move |ctx: JSContext<V::Context>| {
            let obj = Class::get::<JSAsyncIterator<V, T>>(&ctx)?
                .instance(JSAsyncIterator::new(stream_rc.clone(), &ctx));

            let next = JSFunc::new(
                &ctx,
                async move |this: ThisMut<JSAsyncIterator<V, T>>, ctx: JSContext<V::Context>| {
                    let result = this.result.clone();

                    let mut stream = this.stream.lock().await;
                    match stream.next().await {
                        Some(value) => {
                            result.set("done", false)?;
                            let js_value = value.into_js_value(&ctx);
                            let js_value = JSValue::from_raw(&ctx, js_value);
                            result.set("value", js_value)?;
                        }
                        None => {
                            result.set("done", true)?;
                            result.set("value", JSValue::undefined(&ctx))?;
                        }
                    }

                    Ok(result)
                },
            )?;
            obj.set("next", next)?;
            Ok(obj)
        })?;

        // constant of Symbol.asyncIterator
        let symbol = ctx.global().get::<_, JSObject<V>>("Symbol")?;
        let symbol_iterator = symbol.get::<_, JSSymbol<V>>("asyncIterator")?;

        iterable.set(symbol_iterator, async_iterator)?;
        Ok(iterable)
    }
}

/// Auto-implementation of IntoJSAsyncIterator for all types that satisfy the trait bounds
impl<T, S> IntoJSAsyncIterator<T> for S where S: Stream<Item = T> + 'static {}
