use crate::{
    FromJSValue, IntoJSValue, JSArrayBuffer, JSArrayBufferOps, JSContext, JSException, JSObject,
    JSObjectOps, JSResult, JSTypeOf, JSValueImpl, RustyJSError,
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
                println!("line:{}", line!());
                JSObject::from_js_value(ctx, value).map(|obj| Self(obj))
            } else {
                println!("line:{}", line!());
                Err(RustyJSError::NotJSTypedArray)
            }
        } else {
            Err(RustyJSError::NotJSTypedArray)
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
            return Err(RustyJSError::TypedArrayAlignmentError);
        }

        // Check if byte_offset is valid
        let buffer_size = buffer.len();
        if byte_offset >= buffer_size {
            return Err(RustyJSError::TypedArrayRangeError);
        }

        // Calculate maximum possible length
        let max_length = (buffer_size - byte_offset) / bytes_per_element;

        // Validate length
        let length = length.unwrap_or(max_length);
        if length > max_length {
            return Err(RustyJSError::TypedArrayRangeError);
        }

        // Reject empty arrays
        if length == 0 {
            return Err(RustyJSError::TypedArrayRangeError);
        }

        let buffer_value = buffer.into_js_value(ctx);
        let value = V::from_array_buffer(
            ctx.as_ref(),
            T::TYPE,
            buffer_value,
            byte_offset,
            Some(length),
        );
        if value.is_exception() {
            let err = JSException::from_js_value(ctx, value)?;
            Err(RustyJSError::Exception(err.into_error()))
        } else {
            Self::from_js_value(ctx, value)
        }
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
            .ok_or(RustyJSError::NotJSArrayBuffer)?;
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
}

// blanket implementing.
impl<V: JSValueImpl> crate::function::JSParameterType for JSTypedArray<V> {}
