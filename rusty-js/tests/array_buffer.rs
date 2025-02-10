use rustyjs_test::*;

#[test]
fn test_array_buffer_creation() {
    run(|ctx| {
        // Test empty buffer
        let empty_buffer: JSArrayBuffer<u8> = JSArrayBuffer::from_bytes(ctx, &[]).unwrap();
        assert_eq!(empty_buffer.len(), 0);
        assert!(empty_buffer.is_empty());

        // Test normal buffer
        let data = vec![1, 2, 3, 4, 5];
        let buffer: JSArrayBuffer<u8> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        assert_eq!(buffer.len(), 5);
        assert!(!buffer.is_empty());

        // Test buffer with different element types
        let data = vec![1, 0, 2, 0]; // Two 16-bit integers: [1, 2]
        let buffer: JSArrayBuffer<i16> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        assert_eq!(buffer.len(), 4);
        assert_eq!(buffer.element_count(), 2);
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
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];

        // u8 alignment (should always work)
        let buffer: JSArrayBuffer<u8> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        assert!(buffer.validate_alignment(0));
        assert!(buffer.validate_alignment(1));
        assert!(buffer.validate_alignment(2));

        // i16 alignment (must be multiple of 2)
        let buffer: JSArrayBuffer<i16> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        assert!(buffer.validate_alignment(0));
        assert!(!buffer.validate_alignment(1));

        // i32 alignment (must be multiple of 4)
        let buffer: JSArrayBuffer<i32> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        assert!(buffer.validate_alignment(0));
        assert!(!buffer.validate_alignment(1));
    });
}

#[test]
fn test_array_buffer_slice() {
    run(|ctx| {
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let buffer: JSArrayBuffer<u8> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();

        // Test slicing
        let slice = buffer.as_slice();
        assert_eq!(slice, &data[..]);

        // Test bytes
        let bytes = buffer.as_bytes().unwrap();
        assert_eq!(bytes, &data[..]);
    });
}

#[test]
fn test_array_buffer_js_interop() {
    run(|ctx| {
        // Create buffer in Rust
        let data = vec![1, 2, 3, 4];
        let buffer: JSArrayBuffer<u8> = JSArrayBuffer::from_bytes(ctx, &data).unwrap();

        // Set it as a global variable
        let global = ctx.global();
        global.set("testBuffer", buffer);

        // Create a Uint8Array view and modify it in JS
        ctx.eval::<()>(Source::from_bytes(
            b"let view = new Uint8Array(testBuffer); view[0] = 100;",
        ))
        .unwrap();

        // Get it back in Rust and verify the modification
        let modified_buffer: JSArrayBuffer<u8> = ctx.global().get("testBuffer").unwrap();
        let slice = modified_buffer.as_slice();
        assert_eq!(slice[0], 100);
    });
}

#[test]
fn test_array_buffer_zero_copy() {
    run(|ctx| {
        // Create buffer using zero-copy from Vec
        let data = vec![1, 2, 3, 4, 5];
        let buffer: JSArrayBuffer<u8> = JSArrayBuffer::from_bytes_owned(ctx, data).unwrap();

        assert_eq!(buffer.len(), 5);

        // Test with Box<[u8]>
        let boxed_data = vec![6, 7, 8, 9, 10].into_boxed_slice();
        let buffer: JSArrayBuffer<u8> = JSArrayBuffer::from_bytes_owned(ctx, boxed_data).unwrap();

        assert_eq!(buffer.len(), 5);
    });
}

#[test]
fn test_array_buffer_error_cases() {
    run(|ctx| {
        // Test misaligned data for i16
        let data = vec![1, 2, 3]; // 3 bytes is not aligned for i16
        let result: Result<JSArrayBuffer<i16>, _> = JSArrayBuffer::from_bytes(ctx, &data);
        assert!(result.is_err());

        // Test misaligned data for i32
        let data = vec![1, 2, 3, 4, 5, 6]; // 6 bytes is not aligned for i32
        let result: Result<JSArrayBuffer<i32>, _> = JSArrayBuffer::from_bytes(ctx, &data);
        assert!(result.is_err());

        // Test from_object with non-ArrayBuffer
        let obj = JSObject::new(ctx);
        assert!(JSArrayBuffer::<u8>::from_object(obj).is_none());
    });
}
