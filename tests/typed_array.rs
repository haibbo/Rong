use rong_test::*;

#[test]
fn test_typed_array_creation() {
    run(|ctx| {
        // Create an ArrayBuffer to back our TypedArrays
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];

        // Test Int8Array
        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &data)?;
        let int8_array = JSTypedArray::<i8>::from_array_buffer(ctx, buffer.clone(), 0, None)?;
        assert_eq!(int8_array.len(), 8);
        assert_eq!(int8_array.byte_length(), 8);
        assert_eq!(int8_array.bytes_per_element(), 1);
        assert_eq!(int8_array.kind(), JSTypedArrayKind::Int8);

        // Test Int16Array (should have half the length due to 2 bytes per element)
        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &[1, 0, 2, 0, 3, 0, 4, 0])?;
        let int16_array = JSTypedArray::<i16>::from_array_buffer(ctx, buffer.clone(), 0, None)?;
        assert_eq!(int16_array.len(), 4);
        assert_eq!(int16_array.byte_length(), 8);
        assert_eq!(int16_array.bytes_per_element(), 2);
        assert_eq!(int16_array.kind(), JSTypedArrayKind::Int16);

        // Test Int32Array (should have quarter the length due to 4 bytes per element)
        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &[1, 0, 0, 0, 2, 0, 0, 0])?;
        let int32_array = JSTypedArray::<i32>::from_array_buffer(ctx, buffer.clone(), 0, None)?;
        assert_eq!(int32_array.len(), 2);
        assert_eq!(int32_array.byte_length(), 8);
        assert_eq!(int32_array.bytes_per_element(), 4);
        assert_eq!(int32_array.kind(), JSTypedArrayKind::Int32);
        Ok(())
    });
}

#[test]
fn test_typed_array_with_offset_and_length() {
    run(|ctx| {
        // Create a buffer with 16 bytes
        let data = vec![1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7, 0, 8, 0];
        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &data)?;

        // Create Int16Array with offset and length
        let array = JSTypedArray::<i16>::from_array_buffer(
            ctx,
            buffer.clone(),
            2,       // Start at byte offset 2
            Some(3), // Take 3 elements (6 bytes)
        )?;

        assert_eq!(array.byte_offset(), 2);
        assert_eq!(array.len(), 3);
        assert_eq!(array.byte_length(), 6);
        assert_eq!(array.bytes_per_element(), 2);
        Ok(())
    });
}

#[test]
fn test_typed_array_error_handling() {
    run(|ctx| {
        // Test unaligned offset
        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &[1, 0, 2, 0])?;
        let result = JSTypedArray::<i16>::from_array_buffer(
            ctx,
            buffer.clone(),
            1, // Unaligned offset
            None,
        );
        assert!(result.is_err());

        // Test offset beyond buffer size
        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &[1, 2, 3, 4])?;
        let result = JSTypedArray::<i8>::from_array_buffer(
            ctx,
            buffer.clone(),
            5, // Beyond buffer size
            None,
        );
        assert!(result.is_err());

        // Test length too large
        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &[1, 0, 2, 0])?;
        let result = JSTypedArray::<i16>::from_array_buffer(
            ctx,
            buffer.clone(),
            0,
            Some(3), // Would require 6 bytes, but buffer only has 4
        );
        assert!(result.is_err());

        // Test implicit length with misaligned remaining bytes
        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &[1, 2, 3])?;
        let result = JSTypedArray::<i16>::from_array_buffer(ctx, buffer.clone(), 0, None);
        assert!(result.is_err());

        // Test empty array creation
        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &[1, 2, 3, 4])?;
        let result = JSTypedArray::<i8>::from_array_buffer(
            ctx,
            buffer.clone(),
            0,
            Some(0), // Empty array shoud be supported
        );
        assert!(result.is_ok());
        Ok(())
    });
}

#[test]
fn test_typed_array_js_interop() {
    run(|ctx| {
        // Create TypedArray in Rust
        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &[1, 0, 0, 0, 2, 0, 0, 0])?;
        let array = JSTypedArray::<i32>::from_array_buffer(ctx, buffer, 0, None)?;

        // Set it as a global variable
        ctx.global().set("testArray", array)?;

        // Manipulate it in JavaScript
        let sum: i32 = ctx.eval(Source::from_bytes(
            r#"
            let sum = 0;
            for (let i = 0; i < testArray.length; i++) {
                sum += testArray[i];
            }
            sum;
            "#,
        ))?;

        assert_eq!(sum, 3); // 1 + 2 = 3

        // Modify array in JavaScript
        let _: JSValue = ctx.eval(Source::from_bytes(
            r#"
            testArray[0] = 42;
            "#,
        ))?;

        // Get it back in Rust and verify the modification
        let modified_array: JSTypedArray<i32> = ctx.global().get("testArray")?;
        let modified_buffer = modified_array.buffer()?;
        assert_eq!(modified_buffer.as_slice()[0], 42);
        Ok(())
    });
}

#[test]
fn test_all_typed_array_kinds() {
    run(|ctx| {
        // Create a large buffer to accommodate all types
        let mut data = Vec::new();
        for i in 0..32 {
            data.push(i as u8);
        }

        // Test each kind of TypedArray
        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &data)?;
        let array = JSTypedArray::<i8>::from_array_buffer(ctx, buffer.clone(), 0, None)?;
        assert_eq!(array.kind(), JSTypedArrayKind::Int8);

        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &data)?;
        let array = JSTypedArray::<u8>::from_array_buffer(ctx, buffer.clone(), 0, None)?;
        assert_eq!(array.kind(), JSTypedArrayKind::Uint8);

        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &data)?;
        let array = JSTypedArray::<i16>::from_array_buffer(ctx, buffer.clone(), 0, None)?;
        assert_eq!(array.kind(), JSTypedArrayKind::Int16);

        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &data)?;
        let array = JSTypedArray::<u16>::from_array_buffer(ctx, buffer.clone(), 0, None)?;
        assert_eq!(array.kind(), JSTypedArrayKind::Uint16);

        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &data)?;
        let array = JSTypedArray::<i32>::from_array_buffer(ctx, buffer.clone(), 0, None)?;
        assert_eq!(array.kind(), JSTypedArrayKind::Int32);

        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &data)?;
        let array = JSTypedArray::<u32>::from_array_buffer(ctx, buffer.clone(), 0, None)?;
        assert_eq!(array.kind(), JSTypedArrayKind::Uint32);

        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &data)?;
        let array = JSTypedArray::<f32>::from_array_buffer(ctx, buffer.clone(), 0, None)?;
        assert_eq!(array.kind(), JSTypedArrayKind::Float32);

        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &data)?;
        let array = JSTypedArray::<f64>::from_array_buffer(ctx, buffer.clone(), 0, None)?;
        assert_eq!(array.kind(), JSTypedArrayKind::Float64);

        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &data)?;
        let array = JSTypedArray::<i64>::from_array_buffer(ctx, buffer.clone(), 0, None)?;
        assert_eq!(array.kind(), JSTypedArrayKind::BigInt64);

        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &data)?;
        let array = JSTypedArray::<u64>::from_array_buffer(ctx, buffer.clone(), 0, None)?;
        assert_eq!(array.kind(), JSTypedArrayKind::BigUint64);
        Ok(())
    });
}

#[test]
fn test_typed_array_buffer_sharing() {
    run(|ctx| {
        // Create a buffer
        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &[1, 2, 3, 4])?;

        // Create two views of the same buffer
        let array1 = JSTypedArray::<i8>::from_array_buffer(ctx, buffer.clone(), 0, None)?;
        let array2 = JSTypedArray::<i8>::from_array_buffer(ctx, buffer.clone(), 0, None)?;

        // Set them as global variables
        ctx.global().set("array1", array1)?;
        ctx.global().set("array2", array2)?;

        // Modify through one view and check it's visible in the other
        let _: JSValue = ctx.eval(Source::from_bytes(
            r#"
            array1[0] = 100;
            "#,
        ))?;

        let value: i32 = ctx.eval(Source::from_bytes("array2[0]"))?;
        assert_eq!(value, 100);
        Ok(())
    });
}

#[test]
fn test_typed_array_as_bytes() {
    run(|ctx| {
        // Create a buffer with test data
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let buffer: JSArrayBuffer = JSArrayBuffer::from_bytes(ctx, &data)?;

        // Create TypedArray with offset
        let array = JSTypedArray::<u8>::from_array_buffer(
            ctx,
            buffer.clone(),
            2,       // Start at byte offset 2
            Some(3), // Take 3 bytes
        )?;

        // Test as_bytes
        let bytes = array.byte_view().expect("Should return Some bytes");
        assert_eq!(bytes, &[3, 4, 5]);

        // Test with invalid offset
        let invalid_array = JSTypedArray::<u8>::from_array_buffer(
            ctx,
            buffer.clone(),
            10, // Invalid offset beyond buffer size
            None,
        );
        assert!(invalid_array.is_err());

        // Test ArrayBuffer as_bytes
        let buffer_bytes = buffer.as_bytes();
        assert_eq!(buffer_bytes, &[1, 2, 3, 4, 5, 6, 7, 8]);
        Ok(())
    });
}
