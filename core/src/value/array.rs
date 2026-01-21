use crate::{
    FromJSValue, IntoJSValue, JSContext, JSObject, JSObjectOps, JSResult, JSTypeOf, JSValue,
    JSValueConversion, JSValueImpl, JSValueMapper, RongJSError,
};
use std::fmt;
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
    fn into_js_value(self, _ctx: &JSContext<V::Context>) -> JSValue<V> {
        self.0.into_js_value()
    }
}

impl<V> FromJSValue<V> for JSArray<V>
where
    V: JSTypeOf,
{
    fn from_js_value(ctx: &JSContext<V::Context>, value: JSValue<V>) -> JSResult<Self> {
        if value.is_array() {
            JSObject::from_js_value(ctx, value).map(Self)
        } else {
            Err(RongJSError::NotJSArray())
        }
    }
}

/// Trait for JavaScript array operations
pub trait JSArrayOps: JSValueImpl {
    /// Create a new empty array
    fn new(ctx: &Self::Context) -> Self;

    /// Get element at index
    ///
    /// # Returns
    /// Returns the element at the specified index, or an `exception` if failed.
    /// Caller should check if the result is an exception.
    fn get(&self, index: u32) -> Self;

    /// Set element at index
    ///
    /// # Returns
    /// Returns `UNDEFINED` if successful, or an `exception` if failed.
    /// Both are of type `Self`. Caller should check if the result is an exception.
    fn set(&self, index: u32, value: Self) -> Self;
}

impl<V> JSArray<V>
where
    V: JSObjectOps + JSArrayOps,
{
    /// Create a new empty JavaScript array
    pub fn new(ctx: &JSContext<V::Context>) -> JSResult<Self> {
        let value = V::new(ctx.as_ref());
        value.try_map(|v| Self::from_js_value(ctx, JSValue::from_raw(ctx, v)))?
    }

    /// Get the length of the JavaScript array
    pub fn len(&self) -> u32 {
        self.0.get::<_, u32>("length").unwrap_or(0)
    }

    /// Check if the JavaScript array is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get element at index
    ///
    /// # Returns
    /// - `Ok(Some(value))` if element exists and can be converted
    /// - `Ok(None)` if index is out of bounds
    /// - `Err(_)` if type conversion fails or an exception occurred
    pub fn get<T>(&self, index: u32) -> JSResult<Option<T>>
    where
        T: FromJSValue<V>,
    {
        if index >= self.len() {
            return Ok(None);
        }
        let value = self.as_value().get(index);
        let ctx = &self.get_ctx();
        value.try_map(|v| T::from_js_value(ctx, JSValue::from_raw(ctx, v)).map(Some))?
    }

    /// Set element at specified index
    ///
    /// # Arguments
    /// * `index` - The array index to set
    /// * `value` - The value to set at the index
    ///
    /// # Returns
    /// - `Ok(&Self)` if successful, allowing method chaining
    /// - `Err(RongJSError)` if an exception occurred
    pub fn set<T>(&self, index: u32, value: T) -> JSResult<&Self>
    where
        T: IntoJSValue<V>,
    {
        let ctx = self.get_ctx();
        let value = <T as IntoJSValue<V>>::into_js_value(value, &ctx);
        let result = self.as_value().set(index, value.into_value());
        result.try_map(|_| self)
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

    /// Push a value to the end of the array
    ///
    /// # Arguments
    /// * `value` - The value to push
    ///
    /// # Returns
    /// `JSResult<()>` indicating success or failure
    pub fn push<T>(&self, value: T) -> JSResult<()>
    where
        T: IntoJSValue<V>,
    {
        let ctx = self.get_ctx();
        let value = <T as IntoJSValue<V>>::into_js_value(value, &ctx);
        let index = self.len();
        self.as_value().set(index, value.into_value());
        Ok(())
    }

    /// Pop element from end of array
    ///
    /// # Returns
    /// - `Ok(Some(value))` if array not empty and value can be converted
    /// - `Ok(None)` if array is empty
    /// - `Err(_)` if type conversion fails
    pub fn pop<T>(&self) -> JSResult<Option<T>>
    where
        T: FromJSValue<V>,
    {
        if self.is_empty() {
            return Ok(None);
        }

        let index = self.len() - 1;
        let value = self.as_value().get(index);

        // delete the element and update its length
        self.0.del(index);
        self.0.set("length", index)?;

        let ctx = self.get_ctx();
        T::from_js_value(&ctx, JSValue::from_raw(&ctx, value)).map(Some)
    }

    /// Construct a JSArray from a JSObject if it is an array
    ///
    /// # Arguments
    /// * `obj` - The JSObject to check and convert
    ///
    /// # Returns
    /// - `Some(JSArray)` if the object is an array
    /// - `None` if the object is not an array
    pub fn from_object(obj: JSObject<V>) -> Option<Self> {
        if obj.as_value().is_array() {
            Some(Self(obj))
        } else {
            None
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
            let ctx = self.array.get_ctx();
            let res = T::from_js_value(&ctx, JSValue::from_raw(&ctx, v));
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
        (self.count - self.index) as usize
    }
}

/// Converts a Rust Vec into a JavaScript array
impl<V, T> IntoJSValue<V> for Vec<T>
where
    V: JSObjectOps + JSArrayOps,
    T: IntoJSValue<V>,
{
    fn into_js_value(self, ctx: &JSContext<V::Context>) -> JSValue<V> {
        let array = JSArray::new(ctx).unwrap();
        for item in self {
            array.push(item).expect("Failed to set value in array");
        }
        <JSArray<V> as IntoJSValue<V>>::into_js_value(array, ctx)
    }
}

/// Converts a JavaScript array to a Rust Vec
impl<V, T> FromJSValue<V> for Vec<T>
where
    V: JSTypeOf,
    V: JSObjectOps + JSArrayOps,
    T: FromJSValue<V>,
{
    fn from_js_value(ctx: &JSContext<V::Context>, value: JSValue<V>) -> JSResult<Self> {
        if value.is_array() {
            let array = JSArray::from_js_value(ctx, value)?;
            let vec = array.iter::<T>().collect::<JSResult<Vec<_>>>()?;
            Ok(vec)
        } else {
            Err(RongJSError::NotJSArray())
        }
    }
}

// blanket implementing.
// Type JSArray can be as parameter of JS callback of rust function
impl<V: JSValueImpl> crate::function::JSParameterType for JSArray<V> {}

impl<V> fmt::Display for JSArray<V>
where
    V: JSTypeOf + JSValueConversion,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Delegate to JSValue's Display implementation through Deref
        self.0.deref().fmt(f)
    }
}

impl<V> fmt::Debug for JSArray<V>
where
    V: JSTypeOf + JSValueConversion,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSArray({})", self)
    }
}
