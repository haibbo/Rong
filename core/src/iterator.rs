use crate::rong::spawn;
use crate::{IntoJSValue, JSContext, JSFunc, JSObject, JSObjectOps, JSResult, JSSymbol, JSValue};
use futures::{Stream, StreamExt};
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use tokio::sync::Mutex;

/// Core JavaScript iterator that wraps Rust iterators.
///
/// The returned object is both an **iterator** (has `next()`) and an **iterable**
/// (has `[Symbol.iterator]`), following the standard self-referential pattern.
/// This means you can use it directly with `for...of` or call `.next()` manually.
///
/// # Quick usage
/// ```ignore
/// // From any IntoIterator:
/// let iter = vec!["a", "b", "c"].to_js_iter(&ctx)?;
///
/// // Install on an existing object instead:
/// vec!["a", "b", "c"].install_js_iter(&ctx, &my_obj)?;
/// ```
pub struct JSIterator<V, T>
where
    V: JSObjectOps + 'static,
    T: IntoJSValue<V> + 'static,
{
    inner: Rc<RefCell<Box<dyn Iterator<Item = T> + 'static>>>,
    result: JSObject<V>,
}

impl<V, T> JSIterator<V, T>
where
    V: JSObjectOps + 'static,
    T: IntoJSValue<V> + 'static,
{
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

    pub fn next(&self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>> {
        let result = self.result.clone();
        let mut iter = self.inner.borrow_mut();

        match iter.next() {
            Some(item) => {
                result.set("done", false)?;
                let value = <T as IntoJSValue<V>>::into_js_value(item, ctx);
                result.set("value", value)?;
            }
            None => {
                result.set("done", true)?;
                result.set("value", JSValue::undefined(ctx))?;
            }
        }

        Ok(result)
    }

    /// Install `next()` and `[Symbol.iterator]` on an existing JS object.
    pub fn install_on(&self, ctx: &JSContext<V::Context>, obj: &JSObject<V>) -> JSResult<()> {
        let iterator_instance = self.clone();
        let next_fn = JSFunc::new(ctx, move |ctx: JSContext<V::Context>| -> JSObject<V> {
            iterator_instance.next(&ctx).unwrap_or_else(|_| {
                let result = JSObject::new(&ctx);
                result.set("done", true).ok();
                result.set("value", JSValue::undefined(&ctx)).ok();
                result
            })
        })?;
        obj.set("next", next_fn)?;

        let symbol = ctx
            .global()
            .get::<_, JSObject<V>>("Symbol")?
            .get::<_, JSSymbol<V>>("iterator")?;
        let obj_clone = obj.clone();
        obj.set(symbol, JSFunc::new(ctx, move || obj_clone.clone())?)?;
        Ok(())
    }

    /// Create a new JS object that is both iterator and iterable.
    pub fn to_js_iterable(&self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>> {
        let obj = JSObject::new(ctx);
        self.install_on(ctx, &obj)?;
        Ok(obj)
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

/// Core JavaScript async iterator that wraps Rust streams.
///
/// The returned object is both an **async iterator** (has `next()`) and an
/// **async iterable** (has `[Symbol.asyncIterator]`).
///
/// # Quick usage
/// ```ignore
/// // From any Stream:
/// let iter = my_stream.to_js_async_iter(&ctx)?;
///
/// // Install on an existing object (e.g. ReadableStream):
/// my_stream.install_js_async_iter(&ctx, &stream_obj)?;
/// ```
pub struct JSAsyncIterator<V, T>
where
    V: JSObjectOps + 'static,
    T: IntoJSValue<V> + 'static,
{
    stream: Rc<Mutex<Pin<Box<dyn Stream<Item = T> + 'static>>>>,
    result: JSObject<V>,
}

impl<V, T> JSAsyncIterator<V, T>
where
    V: JSObjectOps + 'static,
    T: IntoJSValue<V> + 'static,
{
    pub fn from<S>(stream: S, ctx: &JSContext<V::Context>) -> Self
    where
        S: Stream<Item = T> + 'static,
    {
        Self {
            stream: Rc::new(Mutex::new(Box::pin(stream))),
            result: JSObject::new(ctx),
        }
    }

    pub async fn next(&self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>> {
        let result = self.result.clone();
        let mut stream = self.stream.lock().await;

        match stream.next().await {
            Some(item) => {
                result.set("done", false)?;
                let value = <T as IntoJSValue<V>>::into_js_value(item, ctx);
                result.set("value", value)?;
            }
            None => {
                result.set("done", true)?;
                result.set("value", JSValue::undefined(ctx))?;
            }
        }

        Ok(result)
    }

    /// Install `next()`, `return()`, and `[Symbol.asyncIterator]` on an existing JS object.
    ///
    /// `return()` signals early termination (e.g. `break` in `for await...of`)
    /// by dropping the underlying stream.
    pub fn install_on(&self, ctx: &JSContext<V::Context>, obj: &JSObject<V>) -> JSResult<()> {
        // next()
        let iterator_instance = self.clone();
        let next_fn = JSFunc::new(ctx, move |ctx: JSContext<V::Context>| -> JSObject<V> {
            match ctx.promise() {
                Ok((promise, resolve, _reject)) => {
                    let iter = iterator_instance.clone();
                    spawn(async move {
                        match iter.next(&ctx).await {
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
                    let result = JSObject::new(&ctx);
                    result.set("done", true).ok();
                    result.set("value", JSValue::undefined(&ctx)).ok();
                    result
                }
            }
        })?;
        obj.set("next", next_fn)?;

        // return() — for early termination (break in for-await-of)
        let stream_handle = self.stream.clone();
        let return_fn = JSFunc::new(ctx, move |ctx: JSContext<V::Context>| -> JSObject<V> {
            // Drop the stream to release resources
            let stream = stream_handle.clone();
            match ctx.promise() {
                Ok((promise, resolve, _reject)) => {
                    spawn(async move {
                        // Lock and replace with an empty stream to release resources
                        let mut guard = stream.lock().await;
                        *guard = Box::pin(futures::stream::empty());
                        let result = JSObject::new(&ctx);
                        result.set("done", true).ok();
                        result.set("value", JSValue::undefined(&ctx)).ok();
                        let _ = resolve.call::<_, ()>(None, (result,));
                    });
                    promise.into_object()
                }
                Err(_) => {
                    let result = JSObject::new(&ctx);
                    result.set("done", true).ok();
                    result.set("value", JSValue::undefined(&ctx)).ok();
                    result
                }
            }
        })?;
        obj.set("return", return_fn)?;

        // [Symbol.asyncIterator] = () => this
        let symbol = ctx
            .global()
            .get::<_, JSObject<V>>("Symbol")?
            .get::<_, JSSymbol<V>>("asyncIterator")?;
        let obj_clone = obj.clone();
        obj.set(symbol, JSFunc::new(ctx, move || obj_clone.clone())?)?;
        Ok(())
    }

    /// Create a new JS object that is both async iterator and async iterable.
    pub fn to_js_async_iterable(&self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>> {
        let obj = JSObject::new(ctx);
        self.install_on(ctx, &obj)?;
        Ok(obj)
    }
}

impl<V, T> Clone for JSAsyncIterator<V, T>
where
    V: JSObjectOps + 'static,
    T: IntoJSValue<V> + 'static,
{
    fn clone(&self) -> Self {
        Self {
            stream: self.stream.clone(),
            result: self.result.clone(),
        }
    }
}

/// Extension trait for converting iterables to JavaScript iterators.
pub trait IntoJSIteratorExt<V, T>
where
    V: JSObjectOps + 'static,
{
    /// Create a new JS iterator/iterable object from this Rust iterator.
    fn to_js_iter(self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>>;

    /// Install `next()` and `[Symbol.iterator]` on an existing JS object.
    fn install_js_iter(self, ctx: &JSContext<V::Context>, obj: &JSObject<V>) -> JSResult<()>;
}

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

    fn install_js_iter(self, ctx: &JSContext<V::Context>, obj: &JSObject<V>) -> JSResult<()> {
        let js_iter = JSIterator::from(self, ctx);
        js_iter.install_on(ctx, obj)
    }
}

/// Extension trait for converting streams to JavaScript async iterators.
pub trait IntoJSAsyncIteratorExt<V, T>
where
    V: JSObjectOps + 'static,
{
    /// Create a new JS async iterator/iterable object from this Rust stream.
    fn to_js_async_iter(self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>>;

    /// Install `next()`, `return()`, and `[Symbol.asyncIterator]` on an existing JS object.
    fn install_js_async_iter(self, ctx: &JSContext<V::Context>, obj: &JSObject<V>) -> JSResult<()>;
}

impl<V, T, S> IntoJSAsyncIteratorExt<V, T> for S
where
    V: JSObjectOps + 'static,
    T: IntoJSValue<V> + 'static,
    S: Stream<Item = T> + 'static,
{
    fn to_js_async_iter(self, ctx: &JSContext<V::Context>) -> JSResult<JSObject<V>> {
        let js_iter = JSAsyncIterator::from(self, ctx);
        js_iter.to_js_async_iterable(ctx)
    }

    fn install_js_async_iter(self, ctx: &JSContext<V::Context>, obj: &JSObject<V>) -> JSResult<()> {
        let js_iter = JSAsyncIterator::from(self, ctx);
        js_iter.install_on(ctx, obj)
    }
}

/// Install `[Symbol.asyncIterator]` on an existing JS object (self-referential).
///
/// This is a lightweight helper for objects that already have a `next()` method
/// and just need the symbol to become async-iterable.
pub fn install_async_iterator_symbol<V: JSObjectOps + 'static>(
    ctx: &JSContext<V::Context>,
    obj: &JSObject<V>,
) -> JSResult<()> {
    let symbol = ctx
        .global()
        .get::<_, JSObject<V>>("Symbol")?
        .get::<_, JSSymbol<V>>("asyncIterator")?;
    let obj_clone = obj.clone();
    obj.set(symbol, JSFunc::new(ctx, move || obj_clone.clone())?)?;
    Ok(())
}

/// Install `[Symbol.iterator]` on an existing JS object (self-referential).
///
/// This is a lightweight helper for objects that already have a `next()` method
/// and just need the symbol to become iterable.
pub fn install_iterator_symbol<V: JSObjectOps + 'static>(
    ctx: &JSContext<V::Context>,
    obj: &JSObject<V>,
) -> JSResult<()> {
    let symbol = ctx
        .global()
        .get::<_, JSObject<V>>("Symbol")?
        .get::<_, JSSymbol<V>>("iterator")?;
    let obj_clone = obj.clone();
    obj.set(symbol, JSFunc::new(ctx, move || obj_clone.clone())?)?;
    Ok(())
}
