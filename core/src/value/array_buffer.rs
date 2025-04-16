use crate::{
    FromJSValue, IntoJSValue, JSContext, JSObject, JSObjectOps, JSResult, JSTypeOf, JSValueImpl,
    JSValueMapper, RongJSError, TypedArrayElement,
};

use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

pub struct JSArrayBuffer<V: JSValueImpl, T: TypedArrayElement = u8> {
    inner: JSObject<V>,
    _phantom: PhantomData<T>,
}

impl<V: JSValueImpl, T: TypedArrayElement> Deref for JSArrayBuffer<V, T> {
    type Target = JSObject<V>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<V: JSValueImpl, T: TypedArrayElement> DerefMut for JSArrayBuffer<V, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<V: JSValueImpl, T: TypedArrayElement> Clone for JSArrayBuffer<V, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<V, T> IntoJSValue<V> for JSArrayBuffer<V, T>
where
    V: JSValueImpl,
    T: TypedArrayElement,
{
    fn into_js_value(self, ctx: &JSContext<V::Context>) -> V {
        self.inner.into_js_value(ctx)
    }
}

impl<V, T> FromJSValue<V> for JSArrayBuffer<V, T>
where
    V: JSTypeOf,
    T: TypedArrayElement,
{
    fn from_js_value(ctx: &JSContext<V::Context>, value: V) -> JSResult<Self> {
        if value.is_array_buffer() {
            Ok(Self {
                inner: JSObject::from_js_value(ctx, value)?,
                _phantom: PhantomData,
            })
        } else {
            Err(RongJSError::NotJSArrayBuffer)
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

impl<V, T> JSArrayBuffer<V, T>
where
    V: JSObjectOps + JSArrayBufferOps,
    T: TypedArrayElement,
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
        // Validate that the byte length is a multiple of the element size
        if bytes.len() % T::BYTES_PER_ELEMENT != 0 {
            return Err(RongJSError::TypedArrayAlignmentError);
        }

        let value = V::from_bytes(ctx.as_ref(), bytes);
        value.try_map(|value| Self::from_js_value(ctx, value))?
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
    pub fn from_bytes_owned<B: Into<Vec<u8>>>(
        ctx: &JSContext<V::Context>,
        data: B,
    ) -> JSResult<Self> {
        let vec = data.into();
        // Validate that the byte length is a multiple of the element size
        if vec.len() % T::BYTES_PER_ELEMENT != 0 {
            return Err(RongJSError::TypedArrayAlignmentError);
        }

        let value = V::from_vec(ctx.as_ref(), vec);
        value.try_map(|value| Self::from_js_value(ctx, value))?
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

    /// Get a reference to the array buffer's data as bytes
    ///
    /// # Returns
    /// Returns a slice of the data, or None if:
    /// * The array buffer is detached
    /// * There is any other error accessing the data
    pub fn as_bytes(&self) -> Option<&[u8]> {
        // Always return the slice, even if it's empty
        Some(self.as_slice())
    }

    /// Get a mutable slice view of the ArrayBuffer's data
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        self.as_mut_value().as_mut_slice()
    }

    /// Get a slice of the ArrayBuffer from start to end
    pub fn slice(&self, start: usize, end: usize) -> &[u8] {
        &self.as_slice()[start..end]
    }

    /// Copy the contents of the ArrayBuffer into a new Vec
    pub fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }

    /// Get the number of elements this buffer can hold
    pub fn element_count(&self) -> usize {
        self.len() / T::BYTES_PER_ELEMENT
    }

    /// Validate if the given byte offset is properly aligned for this type
    pub fn validate_alignment(&self, offset: usize) -> bool {
        offset % T::BYTES_PER_ELEMENT == 0
    }

    /// Construct a JSArrayBuffer from a JSObject if it is an ArrayBuffer
    ///
    /// # Arguments
    /// * `obj` - The JSObject to check and convert
    ///
    /// # Returns
    /// - `Some(JSArrayBuffer)` if the object is an ArrayBuffer
    /// - `None` if the object is not an ArrayBuffer
    pub fn from_object(obj: JSObject<V>) -> Option<Self> {
        if obj.as_value().is_array_buffer() {
            Some(Self {
                inner: obj,
                _phantom: PhantomData,
            })
        } else {
            None
        }
    }
}

// blanket implementing.
impl<V: JSValueImpl> crate::function::JSParameterType for JSArrayBuffer<V> {}
