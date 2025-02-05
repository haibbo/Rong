mod helper;
use helper::*;

#[test]
fn test_array_buffer_creation() {
    run(|ctx| {
        // Test creating from &[u8]
        let data = vec![1, 2, 3, 4, 5];
        let buffer = JSArrayBuffer::from_bytes(ctx, &data).unwrap();
        assert_some!(buffer.is_array_buffer());
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.as_slice(), &[1, 2, 3, 4, 5]);

        // Test creating from Vec<u8> (zero-copy)
        let data = vec![6, 7, 8, 9, 10];
        let buffer = JSArrayBuffer::from_bytes_owned(ctx, data).unwrap();
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.as_slice(), &[6, 7, 8, 9, 10]);

        // Test creating from Box<[u8]> (zero-copy)
        let data = vec![11, 12, 13, 14, 15].into_boxed_slice();
        let buffer = JSArrayBuffer::from_bytes_owned(ctx, data).unwrap();
        assert_eq!(buffer.len(), 5);
        assert_eq!(buffer.as_slice(), &[11, 12, 13, 14, 15]);

        // Test error handling for invalid data
        let result = ctx.eval::<JSArrayBuffer>(Source::from_bytes(
            "new ArrayBuffer(-1)", // Invalid size
        ));
        assert!(result.is_err());
    });
}

#[test]
fn test_array_buffer_empty() {
    run(|ctx| {
        // Test empty buffer creation
        let buffer = JSArrayBuffer::from_bytes(ctx, &[]).unwrap();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert_eq!(buffer.as_slice(), &[]);
        assert_eq!(buffer.to_vec(), Vec::<u8>::new());
    });
}

#[test]
fn test_array_buffer_mutations() {
    run(|ctx| {
        // Test mutable access and modifications
        let mut buffer = JSArrayBuffer::from_bytes(ctx, &[1, 2, 3]).unwrap();

        // Test initial state
        assert_eq!(buffer.as_slice(), &[1, 2, 3]);

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
fn test_array_buffer_slicing() {
    run(|ctx| {
        let buffer = JSArrayBuffer::from_bytes(ctx, &[1, 2, 3, 4, 5]).unwrap();

        // Test various slice operations
        assert_eq!(buffer.slice(1, 4), &[2, 3, 4]);
        assert_eq!(buffer.slice(0, 5), &[1, 2, 3, 4, 5]);
        assert_eq!(buffer.slice(2, 3), &[3]);

        // Test edge cases
        assert_eq!(buffer.slice(0, 0), &[]);
        assert_eq!(buffer.slice(5, 5), &[]);

        // Test full buffer access methods
        assert_eq!(buffer.to_bytes(), &[1, 2, 3, 4, 5]);
        assert_eq!(buffer.as_slice(), &[1, 2, 3, 4, 5]);
    });
}

#[test]
fn test_array_buffer_cloning() {
    run(|ctx| {
        let buffer = JSArrayBuffer::from_bytes(ctx, &[1, 2, 3]).unwrap();

        // Test cloning through to_vec
        let cloned = buffer.to_vec();
        assert_eq!(cloned, vec![1, 2, 3]);

        // Verify independence of clone
        let mut cloned = cloned;
        cloned[0] = 42;
        assert_eq!(buffer.as_slice(), &[1, 2, 3]);
        assert_eq!(cloned, vec![42, 2, 3]);

        // Test that original buffer is unchanged
        assert_eq!(buffer.to_bytes(), &[1, 2, 3]);
    });
}

#[test]
fn test_array_buffer_large_data() {
    run(|ctx| {
        // Test with larger data
        let large_data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();
        let buffer = JSArrayBuffer::from_bytes_owned(ctx, large_data.clone()).unwrap();

        // Test size and content
        assert_eq!(buffer.len(), 1000);
        assert_eq!(buffer.as_slice(), large_data.as_slice());

        // Test slicing large data
        assert_eq!(buffer.slice(100, 200), &large_data[100..200]);

        // Test memory efficiency
        let copied = buffer.to_vec();
        assert_eq!(copied.len(), 1000);
        assert_eq!(copied, large_data);
    });
}

#[test]
fn test_array_buffer_js_interop() {
    run(|ctx| {
        // Create buffer in Rust
        let buffer = JSArrayBuffer::from_bytes(ctx, &[1, 2, 3, 4]).unwrap();

        // Set buffer as a global variable for JavaScript to access
        ctx.global().set("testBuffer", buffer);

        // Test reading buffer in JavaScript
        let sum: i32 = ctx
            .eval(Source::from_bytes(
                r#"
                const view = new Uint8Array(testBuffer);
                let sum = 0;
                for (let i = 0; i < view.length; i++) {
                    sum += view[i];
                }
                sum;
                "#,
            ))
            .unwrap();

        assert_eq!(sum, 10); // 1 + 2 + 3 + 4 = 10

        // Test modifying buffer in JavaScript
        let _: JSValue = ctx
            .eval(Source::from_bytes(
                r#"
                const view2 = new Uint8Array(testBuffer);
                view2[0] = 255;
                "#,
            ))
            .unwrap();

        // Get the modified buffer back
        let modified_buffer: JSArrayBuffer = ctx.global().get("testBuffer").unwrap();
        assert_eq!(modified_buffer.as_slice()[0], 255);
        assert_eq!(&modified_buffer.as_slice()[1..], &[2, 3, 4]);
    });
}

#[test]
fn test_array_buffer_error_handling() {
    run(|ctx| {
        // Test invalid slice indices
        let buffer = JSArrayBuffer::from_bytes(ctx, &[1, 2, 3]).unwrap();

        // Verify out-of-bounds and invalid ranges don't panic in release mode
        let result = std::panic::catch_unwind(|| buffer.slice(0, 4));
        assert!(cfg!(debug_assertions) == result.is_err());

        let result = std::panic::catch_unwind(|| buffer.slice(4, 5));
        assert!(cfg!(debug_assertions) == result.is_err());

        let result = std::panic::catch_unwind(|| buffer.slice(2, 1));
        assert!(cfg!(debug_assertions) == result.is_err());

        // Test JavaScript error handling
        let result = ctx.eval::<JSArrayBuffer>(Source::from_bytes(
            r#"
            function createInvalidBuffer() {
                return new ArrayBuffer(-1);
            }
            createInvalidBuffer()
            "#,
        ));
        assert!(result.is_err());
    });
}
