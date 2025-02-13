//! # TextEncoder Implementation
//!
//! This module provides a Rust implementation of the JavaScript `TextEncoder` interface,
//! as defined by the [Encoding Standard](https://encoding.spec.whatwg.org/).
//!
//! ## Features
//!
//! - **UTF-8 Support**: Encodes strings into UTF-8 byte sequences.
//! - **Efficient Encoding**: Provides both `encode` and `encodeInto` methods for different use cases.
//! - **Streaming Support**: `encodeInto` allows partial encoding into existing buffers.
//!
//! ## Limitations
//! - Only UTF-8 encoding is supported. Other encodings will result in a `TypeError`.
//!
//! ## Performance Considerations
//! - `encode` creates a new `Uint8Array` for each call, which may allocate memory.
//! - `encodeInto` allows reusing existing buffers, reducing allocations for repeated operations.

use rusty_js::*;

/// Implementation of the JavaScript `TextEncoder` interface.
/// Encodes strings into UTF-8 byte sequences. Currently supports only UTF-8 encoding.
#[js_class]
pub struct TextEncoder {}

#[js_methods]
impl TextEncoder {
    /// Creates a new `TextEncoder` instance.
    #[js_method(constructor)]
    pub fn new() -> Self {
        Self {}
    }

    /// Gets the encoding used by the encoder (always "utf-8").
    #[js_method(getter, enumerable)]
    pub fn encoding(&self) -> String {
        "utf-8".to_string()
    }

    /// Encodes a string into a `Uint8Array` of UTF-8 bytes.
    ///
    /// # Arguments
    ///
    /// * `input` - The string to encode.
    ///
    /// # Returns
    ///
    /// A `Uint8Array` containing the UTF-8 encoded bytes of the input string.
    #[js_method]
    pub fn encode(&self, ctx: JSContext, input: JSValue) -> JSResult<JSTypedArray> {
        let input = if input.is_undefined() || input.is_null() {
            String::new()
        } else {
            input.try_into::<String>()?
        };
        // Convert the string to UTF-8 bytes
        let bytes = input.as_bytes();

        // Create a buffer with the bytes and create a Uint8Array from it
        let buffer = JSArrayBuffer::from_bytes(&ctx, bytes)?;
        JSTypedArray::from_array_buffer::<u8>(&ctx, buffer, 0, Some(bytes.len()))
    }

    /// Encodes a string into a provided `Uint8Array`, returning the number of bytes read and written.
    ///
    /// # Arguments
    ///
    /// * `input` - The string to encode.
    /// * `dest` - The destination `Uint8Array` to write the encoded bytes into.
    ///
    /// # Returns
    ///
    /// An object containing the number of bytes read and written.
    #[js_method(rename = "encodeInto")]
    pub fn encode_into(&self, ctx: JSContext, input: String, dest: JSObject) -> JSResult<JSObject> {
        // First, check if dest can be converted to JSTypedArray
        if let Some(typed_array) = JSTypedArray::from_object(dest) {
            // Then, check if the typed array is Uint8Array
            if typed_array.kind() == JSTypedArrayKind::Uint8 {
                // Get the underlying buffer and its length
                let buffer_len = typed_array.byte_length();
                let mut buffer = typed_array.buffer()?;
                let buffer_data = buffer.as_mut_slice();

                // Convert input string to UTF-8 bytes
                let input_bytes = input.as_bytes();

                // Calculate how many bytes we can write
                let bytes_to_write = std::cmp::min(buffer_len, input_bytes.len());

                // Copy bytes into destination buffer
                buffer_data[..bytes_to_write].copy_from_slice(&input_bytes[..bytes_to_write]);

                // Create result object with read/written counts
                let result = JSObject::new(&ctx);
                result.set("read", bytes_to_write as f64);
                result.set("written", bytes_to_write as f64);

                return Ok(result);
            }
        }

        // If either check fails, return TypeError
        Err(RustyJSError::TypeError(
            "The \"dest\" argument must be an instance of Uint8Array.".to_string(),
        ))
    }
}

/// Registers the `TextEncoder` class with the JavaScript context.
pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<TextEncoder>();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyjs_test::*;

    #[test]
    fn test_text_encoder() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx).unwrap();

            // Test encoding property
            let result: String = ctx.eval(Source::from_bytes(
                r#"
                const encoder1 = new TextEncoder();
                encoder1.encoding;
                "#,
            ))?;
            assert_eq!(result, "utf-8");

            // Test encoding ASCII string
            let result: bool = ctx.eval(Source::from_bytes(
                r#"
                const encoder2 = new TextEncoder();
                const arr2 = encoder2.encode("hello");
                arr2.byteLength === 5 && arr2 instanceof Uint8Array;
                "#,
            ))?;
            assert!(result);

            // Test encoding empty string
            let result: bool = ctx.eval(Source::from_bytes(
                r#"
                const encoder3 = new TextEncoder();
                const arr3 = encoder3.encode("");
                arr3.byteLength === 0 && arr3 instanceof Uint8Array;
                "#,
            ))?;
            assert!(result);

            // Test encoding non-ASCII string
            let result: bool = ctx.eval(Source::from_bytes(
                r#"
                const encoder4 = new TextEncoder();
                const arr4 = encoder4.encode("你好");
                arr4.byteLength === 6 && arr4 instanceof Uint8Array;
                "#,
            ))?;
            assert!(result);

            // Test encoding non-ASCII string
            let result: bool = ctx.eval(Source::from_bytes(
                r#"
                const encoder5 = new TextEncoder();
                const dest1 = new Uint8Array(5);
                const result1 = encoder5.encodeInto('hello', dest1);
                result1.read === 5 && result1.written === 5 && dest1[0] === 104 && dest1[1] === 101 && dest1[2] === 108 && dest1[3] === 108 && dest1[4] === 111;
                "#,
            ))?;
            assert!(result);

            // Test encodeInto with larger destination
            let result: bool = ctx.eval(Source::from_bytes(
                r#"
                const encoder6 = new TextEncoder();
                const dest2 = new Uint8Array(10);
                const result2 = encoder6.encodeInto('hello', dest2);
                result2.read === 5 && result2.written === 5 && dest2[0] === 104 && dest2[1] === 101 && dest2[2] === 108 && dest2[3] === 108 && dest2[4] === 111;
                "#,
            ))?;
            assert!(result);

            // Test encodeInto with smaller destination
            let result: bool = ctx.eval(Source::from_bytes(
                r#"
                const encoder7 = new TextEncoder();
                const dest3 = new Uint8Array(3);
                const result3 = encoder7.encodeInto('hello', dest3);
                result3.read === 3 && result3.written === 3 && dest3[0] === 104 && dest3[1] === 101 && dest3[2] === 108;
                "#,
            ))?;
            assert!(result);

            // Test encodeInto with Uint16Array (should throw TypeError)
            let result: bool = ctx.eval(Source::from_bytes(
                r#"
                const encoder8 = new TextEncoder();
                const dest4 = new Uint16Array(5);
                try {
                    encoder8.encodeInto('hello', dest4);
                    false; // Should not reach here
                } catch (e) {
                    e instanceof TypeError;
                }
                "#,
            ))?;
            assert!(result);

            Ok(())
        });
    }
}
