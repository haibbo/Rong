use crate::{
    FromJSValue, IntoJSValue, JSContext, JSObject, JSObjectOps, JSResult, JSTypeOf, JSValueImpl,
    RustyJSError,
};
use std::marker::PhantomData;
use std::ops::Deref;

pub struct JSArray<V: JSValueImpl>(JSObject<V>);

impl<V: JSValueImpl> Deref for JSArray<V> {
    type Target = JSObject<V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V: JSValueImpl> Clone for JSArray<V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<V> IntoJSValue<V> for JSArray<V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, ctx: &JSContext<V::Context>) -> V {
        self.0.into_js_value(ctx)
    }
}

impl<V> FromJSValue<V> for JSArray<V>
where
    V: JSTypeOf,
{
    fn from_js_value(ctx: &JSContext<V::Context>, value: V) -> JSResult<Self> {
        if value.is_array() {
            JSObject::from_js_value(ctx, value).map(|obj| Self(obj))
        } else {
            Err(RustyJSError::NotJSArray)
        }
    }
}

/// Trait for JavaScript array operations
pub trait JSArrayOps: JSValueImpl {
    /// Create a new empty array
    fn new(ctx: &Self::Context) -> Self;

    /// Get element at index
    fn get(&self, index: u32) -> Self;

    /// Set element at index
    fn set(&self, index: u32, value: Self);
}

impl<V> JSArray<V>
where
    V: JSObjectOps + JSArrayOps,
{
    /// Create a new empty JavaScript array
    pub fn new(ctx: &JSContext<V::Context>) -> JSResult<Self> {
        let v = JSArrayOps::new(ctx.as_ref());
        JSArray::from_js_value(ctx, v)
    }

    /// Get the length of the JavaScript array
    pub fn len(&self) -> u32 {
        self.0.get::<_, u32>("length").unwrap_or(0)
    }

    /// Check if the JavaScript array is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get element at specified index
    ///
    /// # Arguments
    /// * `index` - The array index to retrieve
    ///
    /// # Returns
    /// `JSResult<T>` where T is the type of the element
    pub fn get<T>(&self, index: u32) -> JSResult<T>
    where
        T: FromJSValue<V>,
    {
        let v = self.as_value().get(index);
        T::from_js_value(&self.0.get_ctx(), v)
    }

    /// Set element at specified index
    ///
    /// # Arguments
    /// * `index` - The array index to set
    /// * `value` - The value to set at the index
    pub fn set<T>(&self, index: u32, value: T) -> JSResult<()>
    where
        T: IntoJSValue<V>,
    {
        let ctx = self.get_ctx();
        let v = value.into_js_value(&ctx);
        self.as_value().set(index, v);
        Ok(())
    }

    /// Create an iterator over the array elements
    ///
    /// # Returns
    /// `ArrayIter` that yields elements of type `T`
    pub fn iter<T>(&self) -> ArrayIter<V, T>
    where
        T: FromJSValue<V>,
    {
        let count = self.len();
        ArrayIter {
            array: self.clone(),
            index: 0,
            count,
            marker: PhantomData,
        }
    }
}

/// The iterator for JS Array
pub struct ArrayIter<V, T>
where
    V: JSObjectOps + JSArrayOps,
    T: FromJSValue<V>,
{
    array: JSArray<V>,
    index: u32,
    count: u32,
    marker: PhantomData<T>,
}

impl<V, T> Iterator for ArrayIter<V, T>
where
    V: JSObjectOps + JSArrayOps,
    T: FromJSValue<V>,
{
    type Item = JSResult<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.count {
            let v = self.array.as_value().get(self.index);
            let res = T::from_js_value(&self.array.get_ctx(), v);
            self.index += 1;
            Some(res)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
}

impl<V, T> ExactSizeIterator for ArrayIter<V, T>
where
    V: JSObjectOps + JSArrayOps,
    T: FromJSValue<V>,
{
    fn len(&self) -> usize {
        (self.count - self.index) as _
    }
}

// blanket implementing.
// Type JSFunc can be as parameter of JS callback of rust function
impl<V: JSValueImpl> crate::function::JSParameterType for JSArray<V> {}
