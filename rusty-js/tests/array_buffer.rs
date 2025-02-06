use rustyjs_test::*;

#[test]
fn test_array_buffer_creation() {
    run(|ctx| {
        // Test creating from &[u8] with different element types
        let data = vec![1, 2, 3, 4];

        // Test u8 buffer (should work directly)
        let buffer: JSArrayBuffer<u8> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        assert_some!(buffer.is_array_buffer());
        assert_eq!(buffer.len(), 4);
        assert_eq!(buffer.element_count(), 4);
        assert_eq!(buffer.as_slice(), &[1, 2, 3, 4]);

        // Test i16 buffer (requires aligned data)
        let i16_data = vec![1, 0, 2, 0, 3, 0, 4, 0];
        let buffer: JSArrayBuffer<i16> = JSArrayBuffer::from_bytes(ctx, &i16_data).unwrap();
        assert_eq!(buffer.len(), 8);
        assert_eq!(buffer.element_count(), 4);
        assert_eq!(buffer.as_slice(), &i16_data);

        // Test i32 buffer (requires aligned data)
        let i32_data = vec![1, 0, 0, 0, 2, 0, 0, 0];
        let buffer: JSArrayBuffer<i32> = JSArrayBuffer::from_bytes(ctx, &i32_data).unwrap();
        assert_eq!(buffer.len(), 8);
        assert_eq!(buffer.element_count(), 2);
        assert_eq!(buffer.as_slice(), &i32_data);

        // Test error case with misaligned data
        let result: JSResult<JSArrayBuffer<i16>> = JSArrayBuffer::from_bytes(ctx, &[1, 2, 3]); // 3 bytes for i16
        assert!(result.is_err());
    });
}

#[test]
fn test_array_buffer_empty() {
    run(|ctx| {
        // Test empty buffer creation
        let buffer: JSArrayBuffer<u8> = JSArrayBuffer::from_bytes(ctx, &[]).unwrap();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert_eq!(buffer.element_count(), 0);
        assert_eq!(buffer.as_slice(), &[]);
        assert_eq!(buffer.to_vec(), Vec::<u8>::new());
    });
}

#[test]
fn test_array_buffer_mutations() {
    run(|ctx| {
        // Test mutable access and modifications
        let mut buffer: JSArrayBuffer<u8> = JSArrayBuffer::from_bytes(ctx, &[1, 2, 3]).unwrap();

        // Test initial state
        assert_eq!(buffer.as_slice(), &[1, 2, 3]);
        assert_eq!(buffer.element_count(), 3);

        // Modify through mutable slice
        {
            let slice = buffer.as_mut_slice();
            slice[0] = 42;
            slice[1] = 43;
            slice[2] = 44;
        }

        // Verify modifications
        assert_eq!(buffer.as_slice(), &[42, 43, 44]);

        // Test to_vec creates independent copy
        let vec_copy = buffer.to_vec();
        assert_eq!(vec_copy, vec![42, 43, 44]);
    });
}

#[test]
fn test_array_buffer_alignment() {
    run(|ctx| {
        // Create buffers with different alignments
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];

        // Test i16 alignment
        let buffer: JSArrayBuffer<i16> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        assert!(buffer.validate_alignment(0));
        assert!(buffer.validate_alignment(2));
        assert!(!buffer.validate_alignment(1));
        assert!(!buffer.validate_alignment(3));

        // Test i32 alignment
        let buffer: JSArrayBuffer<i32> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        assert!(buffer.validate_alignment(0));
        assert!(buffer.validate_alignment(4));
        assert!(!buffer.validate_alignment(1));
        assert!(!buffer.validate_alignment(2));

        // Test element count
        assert_eq!(buffer.element_count(), 2); // 8 bytes = 2 i32s
    });
}

#[test]
fn test_array_buffer_js_interop() {
    run(|ctx| {
        // Create TypedArray in Rust - using native endian to match JS
        let value1 = 1i32.to_ne_bytes();
        let value2 = 2i32.to_ne_bytes();
        let mut data = Vec::new();
        data.extend_from_slice(&value1);
        data.extend_from_slice(&value2);

        let buffer: JSArrayBuffer<i32> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
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
        let bytes = modified_buffer.as_slice();

        // Read using native endian to match JS
        let value = i32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        assert_eq!(value, 42);
    });
}
