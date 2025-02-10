use rustyjs_test::*;

#[test]
fn test_typed_array_creation() {
    run(|ctx| {
        // Create an ArrayBuffer to back our TypedArrays
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];

        // Test Int8Array
        let buffer: JSArrayBuffer<i8> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        let int8_array = JSTypedArray::from_array_buffer(ctx, buffer.clone(), 0, None).unwrap();
        assert_eq!(int8_array.len(), 8);
        assert_eq!(int8_array.byte_length(), 8);
        assert_eq!(int8_array.bytes_per_element(), 1);
        assert_eq!(int8_array.kind(), JSTypedArrayKind::Int8);

        // Test Int16Array (should have half the length due to 2 bytes per element)
        let buffer: JSArrayBuffer<i16> =
            JSArrayBuffer::from_bytes(ctx, &[1, 0, 2, 0, 3, 0, 4, 0]).unwrap();
        let int16_array = JSTypedArray::from_array_buffer(ctx, buffer.clone(), 0, None).unwrap();
        assert_eq!(int16_array.len(), 4);
        assert_eq!(int16_array.byte_length(), 8);
        assert_eq!(int16_array.bytes_per_element(), 2);
        assert_eq!(int16_array.kind(), JSTypedArrayKind::Int16);

        // Test Int32Array (should have quarter the length due to 4 bytes per element)
        let buffer: JSArrayBuffer<i32> =
            JSArrayBuffer::from_bytes(ctx, &[1, 0, 0, 0, 2, 0, 0, 0]).unwrap();
        let int32_array = JSTypedArray::from_array_buffer(ctx, buffer.clone(), 0, None).unwrap();
        assert_eq!(int32_array.len(), 2);
        assert_eq!(int32_array.byte_length(), 8);
        assert_eq!(int32_array.bytes_per_element(), 4);
        assert_eq!(int32_array.kind(), JSTypedArrayKind::Int32);
    });
}

#[test]
fn test_typed_array_with_offset_and_length() {
    run(|ctx| {
        // Create a buffer with 16 bytes
        let data = vec![1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7, 0, 8, 0];
        let buffer: JSArrayBuffer<i16> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();

        // Create Int16Array with offset and length
        let array = JSTypedArray::from_array_buffer(
            ctx,
            buffer.clone(),
            2,       // Start at byte offset 2
            Some(3), // Take 3 elements (6 bytes)
        )
        .unwrap();

        assert_eq!(array.byte_offset(), 2);
        assert_eq!(array.len(), 3);
        assert_eq!(array.byte_length(), 6);
        assert_eq!(array.bytes_per_element(), 2);
    });
}

#[test]
fn test_typed_array_error_handling() {
    run(|ctx| {
        // Test unaligned offset
        let buffer: JSArrayBuffer<i16> = JSArrayBuffer::from_bytes(ctx, &[1, 0, 2, 0]).unwrap();
        let result = JSTypedArray::from_array_buffer(
            ctx,
            buffer.clone(),
            1, // Unaligned offset
            None,
        );
        assert!(result.is_err());

        // Test offset beyond buffer size
        let buffer: JSArrayBuffer<i8> = JSArrayBuffer::from_bytes(ctx, &[1, 2, 3, 4]).unwrap();
        let result = JSTypedArray::from_array_buffer(
            ctx,
            buffer.clone(),
            5, // Beyond buffer size
            None,
        );
        assert!(result.is_err());

        // Test length too large
        let buffer: JSArrayBuffer<i16> = JSArrayBuffer::from_bytes(ctx, &[1, 0, 2, 0]).unwrap();
        let result = JSTypedArray::from_array_buffer(
            ctx,
            buffer.clone(),
            0,
            Some(3), // Would require 6 bytes, but buffer only has 4
        );
        assert!(result.is_err());

        // Test empty array creation (should fail)
        let buffer: JSArrayBuffer<i8> = JSArrayBuffer::from_bytes(ctx, &[1, 2, 3, 4]).unwrap();
        let result = JSTypedArray::from_array_buffer(
            ctx,
            buffer.clone(),
            0,
            Some(0), // Empty array not supported
        );
        assert!(result.is_err());
    });
}

#[test]
fn test_typed_array_js_interop() {
    run(|ctx| {
        // Create TypedArray in Rust
        let buffer: JSArrayBuffer<i32> =
            JSArrayBuffer::from_bytes(ctx, &[1, 0, 0, 0, 2, 0, 0, 0]).unwrap();
        let array = JSTypedArray::from_array_buffer(ctx, buffer, 0, None).unwrap();

        // Set it as a global variable
        ctx.global().set("testArray", array);

        // Manipulate it in JavaScript
        let sum: i32 = ctx
            .eval(Source::from_bytes(
                r#"
            let sum = 0;
            for (let i = 0; i < testArray.length; i++) {
                sum += testArray[i];
            }
            sum;
            "#,
            ))
            .unwrap();

        assert_eq!(sum, 3); // 1 + 2 = 3

        // Modify array in JavaScript
        let _: JSValue = ctx
            .eval(Source::from_bytes(
                r#"
            testArray[0] = 42;
            "#,
            ))
            .unwrap();

        // Get it back in Rust and verify the modification
        let modified_array: JSTypedArray = ctx.global().get("testArray").unwrap();
        let modified_buffer = modified_array.buffer().unwrap();
        assert_eq!(modified_buffer.as_slice()[0], 42);
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
        let buffer: JSArrayBuffer<i8> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        let array = JSTypedArray::from_array_buffer(ctx, buffer.clone(), 0, None).unwrap();
        assert_eq!(array.kind(), JSTypedArrayKind::Int8);

        let buffer: JSArrayBuffer<u8> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        let array = JSTypedArray::from_array_buffer(ctx, buffer.clone(), 0, None).unwrap();
        assert_eq!(array.kind(), JSTypedArrayKind::Uint8);

        let buffer: JSArrayBuffer<i16> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        let array = JSTypedArray::from_array_buffer(ctx, buffer.clone(), 0, None).unwrap();
        assert_eq!(array.kind(), JSTypedArrayKind::Int16);

        let buffer: JSArrayBuffer<u16> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        let array = JSTypedArray::from_array_buffer(ctx, buffer.clone(), 0, None).unwrap();
        assert_eq!(array.kind(), JSTypedArrayKind::Uint16);

        let buffer: JSArrayBuffer<i32> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        let array = JSTypedArray::from_array_buffer(ctx, buffer.clone(), 0, None).unwrap();
        assert_eq!(array.kind(), JSTypedArrayKind::Int32);

        let buffer: JSArrayBuffer<u32> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        let array = JSTypedArray::from_array_buffer(ctx, buffer.clone(), 0, None).unwrap();
        assert_eq!(array.kind(), JSTypedArrayKind::Uint32);

        let buffer: JSArrayBuffer<f32> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        let array = JSTypedArray::from_array_buffer(ctx, buffer.clone(), 0, None).unwrap();
        assert_eq!(array.kind(), JSTypedArrayKind::Float32);

        let buffer: JSArrayBuffer<f64> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        let array = JSTypedArray::from_array_buffer(ctx, buffer.clone(), 0, None).unwrap();
        assert_eq!(array.kind(), JSTypedArrayKind::Float64);

        let buffer: JSArrayBuffer<i64> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        let array = JSTypedArray::from_array_buffer(ctx, buffer.clone(), 0, None).unwrap();
        assert_eq!(array.kind(), JSTypedArrayKind::BigInt64);

        let buffer: JSArrayBuffer<u64> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        let array = JSTypedArray::from_array_buffer(ctx, buffer.clone(), 0, None).unwrap();
        assert_eq!(array.kind(), JSTypedArrayKind::BigUint64);
    });
}

#[test]
fn test_typed_array_buffer_sharing() {
    run(|ctx| {
        // Create a buffer
        let buffer: JSArrayBuffer<i8> = JSArrayBuffer::from_bytes(ctx, &[1, 2, 3, 4]).unwrap();

        // Create two views of the same buffer
        let array1 = JSTypedArray::from_array_buffer(ctx, buffer.clone(), 0, None).unwrap();
        let array2 = JSTypedArray::from_array_buffer(ctx, buffer.clone(), 0, None).unwrap();

        // Set them as global variables
        ctx.global().set("array1", array1);
        ctx.global().set("array2", array2);

        // Modify through one view and check it's visible in the other
        let _: JSValue = ctx
            .eval(Source::from_bytes(
                r#"
            array1[0] = 100;
            "#,
            ))
            .unwrap();

        let value: i32 = ctx.eval(Source::from_bytes("array2[0]")).unwrap();
        assert_eq!(value, 100);
    });
}

#[test]
fn test_typed_array_as_bytes() {
    run(|ctx| {
        // Create a buffer with test data
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let buffer: JSArrayBuffer<u8> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();

        // Create TypedArray with offset
        let array = JSTypedArray::from_array_buffer(
            ctx,
            buffer.clone(),
            2,       // Start at byte offset 2
            Some(3), // Take 3 bytes
        )
        .unwrap();

        // Test as_bytes
        let bytes = array.as_bytes().expect("Should return Some bytes");
        assert_eq!(bytes, &[3, 4, 5]);

        // Test with invalid offset
        let invalid_array = JSTypedArray::from_array_buffer(
            ctx,
            buffer.clone(),
            10, // Invalid offset beyond buffer size
            None,
        );
        assert!(invalid_array.is_err());

        // Test ArrayBuffer as_bytes
        let buffer_bytes = buffer.as_bytes().expect("Should return Some bytes");
        assert_eq!(buffer_bytes, &[1, 2, 3, 4, 5, 6, 7, 8]);
    });
}
