use crate::{
    FromJSValue, IntoJSValue, JSContext, JSObject, JSObjectOps, JSResult, JSTypeOf, JSValueImpl,
    RustyJSError,
};

use std::ops::{Deref, DerefMut};

pub struct JSArrayBuffer<V: JSValueImpl>(JSObject<V>);

impl<V: JSValueImpl> Deref for JSArrayBuffer<V> {
    type Target = JSObject<V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V: JSValueImpl> DerefMut for JSArrayBuffer<V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<V> IntoJSValue<V> for JSArrayBuffer<V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, ctx: &JSContext<V::Context>) -> V {
        self.0.into_js_value(ctx)
    }
}

impl<V> FromJSValue<V> for JSArrayBuffer<V>
where
    V: JSTypeOf,
{
    fn from_js_value(ctx: &JSContext<V::Context>, value: V) -> JSResult<Self> {
        if value.is_array_buffer() {
            JSObject::from_js_value(ctx, value).map(|obj| Self(obj))
        } else {
            Err(RustyJSError::NotJSArrayBuffer)
        }
    }
}

/// Trait for JavaScript array buffer operations
pub trait JSArrayBufferOps: JSValueImpl {
    /// Create an ArrayBuffer by copying existing data
    fn from_bytes(ctx: &Self::Context, bytes: &[u8]) -> Self;

    /// Create an ArrayBuffer from an existing Vec without copying (zero-copy)
    ///
    /// # Note
    /// This method takes ownership of the Vec and transfers its memory to the ArrayBuffer.
    /// The Vec's memory will be freed when the ArrayBuffer is garbage collected.
    fn from_vec(ctx: &Self::Context, vec: Vec<u8>) -> Self;

    /// Get the byte length of the ArrayBuffer
    fn length(&self) -> usize;

    /// Get a safe slice view of the ArrayBuffer's data
    fn as_slice(&self) -> &[u8];

    /// Get a mutable slice view of the ArrayBuffer's data
    fn as_mut_slice(&mut self) -> &mut [u8];
}

impl<V> JSArrayBuffer<V>
where
    V: JSObjectOps + JSArrayBufferOps,
{
    /// Create a new ArrayBuffer by copying the provided bytes
    ///
    /// This method always copies the input data into a new ArrayBuffer.
    /// If you have owned data and want to avoid copying, use `from_bytes_owned` instead.
    ///
    /// # Examples
    /// ```
    /// let buffer = JSArrayBuffer::from_bytes(ctx, &[1, 2, 3])?;
    /// ```
    pub fn from_bytes(ctx: &JSContext<V::Context>, bytes: &[u8]) -> JSResult<Self> {
        let value = V::from_bytes(ctx.as_ref(), bytes);
        Self::from_js_value(ctx, value)
    }

    /// Create a new ArrayBuffer from owned bytes
    ///
    /// This method accepts any type that can be converted into Vec<u8>.
    /// When possible, it will use zero-copy optimization by taking ownership
    /// of the underlying memory.
    ///
    /// # Examples
    /// ```
    /// // From Vec (zero-copy)
    /// let buffer = JSArrayBuffer::from_bytes_owned(ctx, vec![1, 2, 3])?;
    ///
    /// // From Box<[u8]> (zero-copy)
    /// let buffer = JSArrayBuffer::from_bytes_owned(ctx, vec![1, 2, 3].into_boxed_slice())?;
    ///
    /// // From &[u8] (will copy)
    /// let buffer = JSArrayBuffer::from_bytes_owned(ctx, &[1, 2, 3].to_vec())?;
    /// ```
    pub fn from_bytes_owned<T: Into<Vec<u8>>>(
        ctx: &JSContext<V::Context>,
        data: T,
    ) -> JSResult<Self> {
        let vec = data.into();
        let value = V::from_vec(ctx.as_ref(), vec);
        Self::from_js_value(ctx, value)
    }

    /// Get the byte length of the ArrayBuffer
    pub fn len(&self) -> usize {
        self.as_value().length()
    }

    /// Check if the ArrayBuffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a safe slice view of the ArrayBuffer's data
    pub fn as_slice(&self) -> &[u8] {
        self.as_value().as_slice()
    }

    /// Get a mutable slice view of the ArrayBuffer's data
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.as_mut_value().as_mut_slice()
    }

    /// Get a slice of the ArrayBuffer from start to end
    pub fn slice(&self, start: usize, end: usize) -> &[u8] {
        &self.as_slice()[start..end]
    }

    /// Get the entire contents of the ArrayBuffer as a byte slice
    pub fn to_bytes(&self) -> &[u8] {
        self.as_slice()
    }

    /// Copy the contents of the ArrayBuffer into a new Vec
    pub fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
}
