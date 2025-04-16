use crate::{
    FromJSValue, IntoJSValue, JSArrayBuffer, JSArrayBufferOps, JSContext, JSObject, JSObjectOps,
    JSResult, JSTypeOf, JSValueImpl, JSValueMapper, RongJSError,
};
use std::ops::Deref;

/// Represents the different kinds of TypedArrays available in JavaScript
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JSTypedArrayKind {
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    BigInt64,
    BigUint64,
    Float32,
    Float64,
}

/// Trait for types that can be used in TypedArrays
pub trait TypedArrayElement: Sized {
    /// Number of bytes per element
    const BYTES_PER_ELEMENT: usize;
    /// The corresponding TypedArray kind
    const TYPE: JSTypedArrayKind;
}

// Implement for all supported types
impl TypedArrayElement for i8 {
    const BYTES_PER_ELEMENT: usize = 1;
    const TYPE: JSTypedArrayKind = JSTypedArrayKind::Int8;
}

impl TypedArrayElement for u8 {
    const BYTES_PER_ELEMENT: usize = 1;
    const TYPE: JSTypedArrayKind = JSTypedArrayKind::Uint8;
}

impl TypedArrayElement for i16 {
    const BYTES_PER_ELEMENT: usize = 2;
    const TYPE: JSTypedArrayKind = JSTypedArrayKind::Int16;
}

impl TypedArrayElement for u16 {
    const BYTES_PER_ELEMENT: usize = 2;
    const TYPE: JSTypedArrayKind = JSTypedArrayKind::Uint16;
}

impl TypedArrayElement for i32 {
    const BYTES_PER_ELEMENT: usize = 4;
    const TYPE: JSTypedArrayKind = JSTypedArrayKind::Int32;
}

impl TypedArrayElement for u32 {
    const BYTES_PER_ELEMENT: usize = 4;
    const TYPE: JSTypedArrayKind = JSTypedArrayKind::Uint32;
}

impl TypedArrayElement for f32 {
    const BYTES_PER_ELEMENT: usize = 4;
    const TYPE: JSTypedArrayKind = JSTypedArrayKind::Float32;
}

impl TypedArrayElement for f64 {
    const BYTES_PER_ELEMENT: usize = 8;
    const TYPE: JSTypedArrayKind = JSTypedArrayKind::Float64;
}

impl TypedArrayElement for i64 {
    const BYTES_PER_ELEMENT: usize = 8;
    const TYPE: JSTypedArrayKind = JSTypedArrayKind::BigInt64;
}

impl TypedArrayElement for u64 {
    const BYTES_PER_ELEMENT: usize = 8;
    const TYPE: JSTypedArrayKind = JSTypedArrayKind::BigUint64;
}

impl JSTypedArrayKind {
    /// Get the number of bytes per element for this type
    pub fn bytes_per_element(&self) -> usize {
        match self {
            JSTypedArrayKind::Int8 | JSTypedArrayKind::Uint8 => 1,
            JSTypedArrayKind::Int16 | JSTypedArrayKind::Uint16 => 2,
            JSTypedArrayKind::Int32 | JSTypedArrayKind::Uint32 | JSTypedArrayKind::Float32 => 4,
            JSTypedArrayKind::BigInt64
            | JSTypedArrayKind::BigUint64
            | JSTypedArrayKind::Float64 => 8,
        }
    }
}

pub struct JSTypedArray<V: JSValueImpl>(JSObject<V>);

impl<V: JSValueImpl> Clone for JSTypedArray<V> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<V: JSValueImpl> Deref for JSTypedArray<V> {
    type Target = JSObject<V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> FromJSValue<V> for JSTypedArray<V>
where
    V: JSTypeOf + JSTypedArrayOps,
{
    fn from_js_value(ctx: &JSContext<V::Context>, value: V) -> JSResult<Self> {
        if value.is_object() {
            if value.get_kind().is_some() {
                JSObject::from_js_value(ctx, value).map(|obj| Self(obj))
            } else {
                Err(RongJSError::NotJSTypedArray)
            }
        } else {
            Err(RongJSError::NotJSTypedArray)
        }
    }
}

impl<V> IntoJSValue<V> for JSTypedArray<V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, ctx: &JSContext<V::Context>) -> V {
        self.0.into_js_value(ctx)
    }
}

/// Trait for JavaScript typed array operations
pub trait JSTypedArrayOps: JSValueImpl {
    /// Create a new typed array from an existing array buffer
    ///
    /// # Arguments
    /// * `ctx` - The JavaScript context
    /// * `kind` - The kind of typed array to create
    /// * `buffer` - The array buffer to use
    /// * `byte_offset` - The offset in bytes from the start of the array buffer
    /// * `length` - Optional number of elements. If None, uses all remaining space in the buffer
    ///
    /// # Returns
    /// Returns an exception if:
    /// * byte_offset is not aligned with the element size
    /// * byte_offset + (length * bytes_per_element) exceeds the buffer's size
    /// * byte_offset is greater than the buffer's size
    fn from_array_buffer(
        ctx: &Self::Context,
        kind: JSTypedArrayKind,
        buffer: Self,
        byte_offset: usize,
        length: Option<usize>,
    ) -> Self;

    /// Get the kind of typed array
    /// Returns None if this is not a typed array
    fn get_kind(&self) -> Option<JSTypedArrayKind>;

    /// Get the underlying array buffer
    fn get_array_buffer(&self) -> Option<Self>;

    /// Get the byte offset into the array buffer
    fn get_byte_offset(&self) -> usize;

    /// Get the length of the typed array (in elements)
    fn get_length(&self) -> usize;

    /// Get the byte length of the typed array
    fn get_byte_length(&self) -> usize;
}

impl<V> JSTypedArray<V>
where
    V: JSObjectOps + JSTypedArrayOps + JSArrayBufferOps,
{
    /// Create a new typed array from an existing array buffer
    pub fn from_array_buffer<T: TypedArrayElement>(
        ctx: &JSContext<V::Context>,
        buffer: JSArrayBuffer<V, T>,
        byte_offset: usize,
        length: Option<usize>,
    ) -> JSResult<Self> {
        // Check alignment
        let bytes_per_element = T::BYTES_PER_ELEMENT;
        if byte_offset % bytes_per_element != 0 {
            return Err(RongJSError::TypedArrayAlignmentError);
        }

        // Check if byte_offset is valid
        let buffer_size = buffer.len();
        if byte_offset > buffer_size {
            return Err(RongJSError::TypedArrayRangeError);
        }

        // Calculate maximum possible length
        let max_length = (buffer_size - byte_offset) / bytes_per_element;

        // Validate length
        let length = length.unwrap_or(max_length);
        if length > max_length {
            return Err(RongJSError::TypedArrayRangeError);
        }

        // Create TypedArray with buffer and offset
        let buffer_value = buffer.into_js_value(ctx);
        let value = V::from_array_buffer(
            ctx.as_ref(),
            T::TYPE,
            buffer_value,
            byte_offset,
            Some(length),
        );
        value.try_map(|value| Self::from_js_value(ctx, value))?
    }

    /// Get the kind of typed array
    pub fn kind(&self) -> JSTypedArrayKind {
        self.as_value().get_kind().expect("Invalid typed array")
    }

    /// Get the underlying array buffer
    pub fn buffer(&self) -> JSResult<JSArrayBuffer<V>> {
        let buffer = self
            .as_value()
            .get_array_buffer()
            .ok_or(RongJSError::NotJSArrayBuffer)?;
        JSArrayBuffer::from_js_value(&self.get_ctx(), buffer)
    }

    /// Get the byte offset into the array buffer
    pub fn byte_offset(&self) -> usize {
        self.as_value().get_byte_offset()
    }

    /// Get the length of the typed array (in elements)
    pub fn len(&self) -> usize {
        self.as_value().get_length()
    }

    /// Check if the typed array is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the byte length of the typed array
    pub fn byte_length(&self) -> usize {
        self.as_value().get_byte_length()
    }

    /// Get the number of bytes per element
    pub fn bytes_per_element(&self) -> usize {
        self.kind().bytes_per_element()
    }

    /// Construct a JSTypedArray from a JSObject if it is a TypedArray
    ///
    /// # Arguments
    /// * `obj` - The JSObject to check and convert
    ///
    /// # Returns
    /// - `Some(JSTypedArray)` if the object is a TypedArray
    /// - `None` if the object is not a TypedArray
    pub fn from_object(obj: JSObject<V>) -> Option<Self> {
        if obj.as_value().get_kind().is_some() {
            Some(Self(obj))
        } else {
            None
        }
    }

    /// Get a reference to the typed array's data
    ///
    /// # Returns
    /// Returns a slice of the data, or None if:
    /// * The array buffer is detached
    /// * The typed array is out of bounds
    /// * There is any other error accessing the data
    pub fn as_bytes(&self) -> Option<&[u8]> {
        let buffer = self.buffer().ok()?;
        let offset = self.byte_offset();
        let length = self.byte_length();

        let buffer_data = buffer.as_bytes()?;
        if offset + length > buffer_data.len() {
            return None;
        }

        // SAFETY: We've verified that:
        // 1. The offset and length are within bounds
        // 2. The buffer is valid and not detached (through as_bytes())
        // 3. The lifetime is tied to self through the buffer
        Some(unsafe { std::slice::from_raw_parts(buffer_data.as_ptr().add(offset), length) })
    }
}

// blanket implementing.
impl<V: JSValueImpl> crate::function::JSParameterType for JSTypedArray<V> {}
