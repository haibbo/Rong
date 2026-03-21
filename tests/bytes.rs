use bytes::Bytes;
use rong_test::*;

#[test]
fn test_js_bytes_rust_construction() {
    run(|ctx| {
        let bytes = JSBytes::from_bytes(ctx, Bytes::from_static(&[1, 2, 3, 4]))?;
        assert_eq!(bytes.len()?, 4);
        assert_eq!(bytes.to_bytes()?, Bytes::from_static(&[1, 2, 3, 4]));
        Ok(())
    });
}

#[test]
fn test_js_bytes_js_constructor_from_string() {
    run(|ctx| {
        let bytes: JSBytes = ctx.eval(Source::from_bytes(
            r#"let payload = new JSBytes('{"type":"ping","id":1}'); payload"#,
        ))?;
        assert_eq!(bytes.to_string()?, r#"{"type":"ping","id":1}"#);

        let length: u32 = ctx.eval(Source::from_bytes(
            r#"new JSBytes('{"type":"ping","id":1}').length"#,
        ))?;
        assert_eq!(length, 22);
        Ok(())
    });
}

#[test]
fn test_js_bytes_constructor_accepts_existing_instance() {
    run(|ctx| {
        let cloned: JSBytes = ctx.eval(Source::from_bytes(
            r#"let first = new JSBytes("hello"); new JSBytes(first)"#,
        ))?;
        assert_eq!(cloned.to_string()?, "hello");
        Ok(())
    });
}

#[test]
fn test_js_bytes_as_rust_func_parameter() {
    run(|ctx| {
        let byte_len = JSFunc::new(ctx, |payload: Bytes| -> usize { payload.len() })?;
        ctx.global().set("byteLen", byte_len)?;

        let len: u32 = ctx.eval(Source::from_bytes(r#"byteLen(new JSBytes("hello"))"#))?;
        assert_eq!(len, 5);
        Ok(())
    });
}
