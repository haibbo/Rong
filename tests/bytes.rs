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
fn test_js_bytes_hidden_from_global() {
    run(|ctx| {
        assert_eq!(
            ctx.eval::<String>(Source::from_bytes(b"typeof JSBytes"))?,
            "undefined"
        );
        assert!(ctx.eval::<bool>(Source::from_bytes(b"globalThis.JSBytes === undefined"))?);
        Ok(())
    });
}

#[test]
fn test_js_bytes_rust_created_instances_work_in_js() {
    run(|ctx| {
        let bytes = JSBytes::from_string(ctx, r#"{"type":"ping","id":1}"#)?;
        ctx.global().set("payload", bytes)?;

        let text: String = ctx.eval(Source::from_bytes(r#"payload.toString()"#))?;
        assert_eq!(text, r#"{"type":"ping","id":1}"#);

        let length: u32 = ctx.eval(Source::from_bytes(r#"payload.length"#))?;
        assert_eq!(length, 22);
        Ok(())
    });
}

#[test]
fn test_js_bytes_cannot_be_constructed_via_instance_constructor() {
    run(|ctx| {
        let payload = JSBytes::from_string(ctx, "hello")?;
        ctx.global().set("payload", payload)?;

        assert!(
            ctx.eval::<JSValue>(Source::from_bytes(r#"new payload.constructor("hello")"#,))
                .is_err()
        );
        assert!(
            ctx.eval::<JSValue>(Source::from_bytes(r#"payload.constructor("hello")"#))
                .is_err()
        );
        Ok(())
    });
}

#[test]
fn test_js_bytes_as_rust_func_parameter() {
    run(|ctx| {
        let payload = JSBytes::from_string(ctx, "hello")?;
        ctx.global().set("payload", payload)?;

        let byte_len = JSFunc::new(ctx, |payload: Bytes| -> usize { payload.len() })?;
        ctx.global().set("byteLen", byte_len)?;

        let len: u32 = ctx.eval(Source::from_bytes(r#"byteLen(payload)"#))?;
        assert_eq!(len, 5);
        Ok(())
    });
}
