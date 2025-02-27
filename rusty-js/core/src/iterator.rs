use crate::{
    IntoJSValue, JSContext, JSExceptionHandler, JSFunc, JSObject, JSObjectOps, JSResult, JSSymbol,
    JSValue,
};

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

// Blanket implementation
impl<T, I> ToJSIterator<T> for I
where
    I: IntoIterator<Item = T>,
    I: Clone,
    I: 'static,
{
}
