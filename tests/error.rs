use rong_test::*;

#[test]
fn test_throw_error() {
    run(|ctx| {
        let error = ctx.throw_syntax_error("Invalid syntax");
        assert!(error.is_exception());

        let error = ctx.throw_type_error("Invalid type");
        assert!(error.is_exception());

        let error = ctx.throw_reference_error("Undefined variable");
        assert!(error.is_exception());

        let error = ctx
            .eval::<()>(Source::from_bytes(b"throw new Error('throw-error')"))
            .unwrap_err();
        let thrown = thrown_js_value(ctx, &error)?;
        let error = String::from_js_value(ctx, thrown.clone())?;
        assert!(
            error.contains("throw-error"),
            "Expected error message to contain 'throw-error', but got: {}",
            error
        );
        Ok(())
    });
}

#[test]
fn test_throw_primitive_value() {
    run(|ctx| {
        let err = ctx.eval::<()>(Source::from_bytes(b"throw 1")).unwrap_err();
        let thrown = thrown_js_value(ctx, &err)?;
        let s = String::from_js_value(ctx, thrown)?;
        assert_eq!(s, "1");
        Ok(())
    });
}

#[test]
fn test_eval_returns_error_object_as_value() {
    run(|ctx| {
        let value: JSValue = ctx.eval(Source::from_bytes(br#"new Error("x")"#))?;
        assert!(value.is_error());
        assert!(!value.is_exception());

        let obj = value
            .into_object()
            .expect("Expected returned Error to be an object");
        let message: String = obj.get("message")?;
        assert_eq!(message, "x");
        Ok(())
    });
}

#[test]
fn test_error_constructor() {
    run(|ctx| {
        // Register multiple error constructors
        ctx.global().set(
            "type_error",
            JSFunc::new(ctx, || -> JSResult<()> {
                Err(
                    HostError::new(rong::error::E_INVALID_ARG, "this is typeError")
                        .with_name("TypeError")
                        .into(),
                )
            })?,
        )?;

        ctx.global().set(
            "reference_error",
            JSFunc::new(ctx, || -> JSResult<()> {
                Err(RongJSError::PropertyNotFound("dummy".to_string()))
            })?,
        )?;

        ctx.global().set(
            "range_error",
            JSFunc::new(ctx, || -> JSResult<()> {
                Err(RongJSError::TypedArrayRangeError())
            })?,
        )?;

        // Test TypeError
        let type_error = ctx
            .eval::<String>(Source::from_bytes(
                b"try {
                    type_error();
                } catch (e) {
                    e.constructor.name + ': ' + e.message
                }",
            ))
            .unwrap();
        assert_eq!(type_error, "TypeError: this is typeError");

        // Test ReferenceError
        let reference_error = ctx
            .eval::<String>(Source::from_bytes(
                b"try {
                    reference_error();
                } catch (e) {
                    e.constructor.name + ': ' + e.message
                }",
            ))
            .unwrap();
        assert_eq!(
            reference_error,
            "ReferenceError: Property 'dummy' Not Found"
        );

        // Test RangeError
        let range_error = ctx
            .eval::<String>(Source::from_bytes(
                b"try {
                    range_error();
                } catch (e) {
                    e.constructor.name + ': ' + e.message
                }",
            ))
            .unwrap();
        assert_eq!(
            range_error,
            "RangeError: Invalid TypedArray range: offset or length exceeds buffer size"
        );
        Ok(())
    });
}

#[test]
fn test_error_stack() {
    run(|ctx| {
        // test syntax error
        let result = ctx.eval::<()>(Source::from_bytes(b"function test() { a b c }"));
        let err = result.unwrap_err();
        let message = thrown_error_message(ctx, &err)?;
        assert!(!message.is_empty(), "Should have error message");
        // Javascriptcore only have value on message
        // assert!(error.stack.is_some(), "Should have stack trace");

        // test Reference Error
        let result = ctx.eval::<()>(Source::from_bytes(
            b"
            function foo() {
                return bar(); // undefined variable
            }
            foo();
        ",
        ));
        let err = result.unwrap_err();
        let message = thrown_error_message(ctx, &err)?;
        assert!(
            message.contains("bar"),
            "Error message should mention undefined variable"
        );
        let stack = thrown_error_stack(ctx, &err)?;
        assert!(
            stack.contains("foo"),
            "Stack trace should contain function name"
        );

        // test type error
        let result = ctx.eval::<()>(Source::from_bytes(
            b"
            let obj = null;
            obj.property;  // TypeError: Cannot read property of null
        ",
        ));
        let err = result.unwrap_err();
        let message = thrown_error_message(ctx, &err)?;
        assert!(
            message.contains("null"),
            "Error message should mention null"
        );
        Ok(())
    });
}

#[test]
fn test_custom_error() {
    run(|ctx| {
        // Test custom errors and stack traces
        let result = ctx.eval::<()>(Source::from_bytes(
            b"
            function throwCustomError() {
                throw new Error('Custom error message');
            }

            function caller() {
                throwCustomError();
            }

            caller();
        ",
        ));

        let err = result.unwrap_err();
        let message = thrown_error_message(ctx, &err)?;
        assert_eq!(message, "Custom error message");
        let stack = thrown_error_stack(ctx, &err)?;
        assert!(
            stack.contains("throwCustomError"),
            "Stack should contain throwCustomError"
        );
        assert!(stack.contains("caller"), "Stack should contain caller");
        Ok(())
    });
}

#[test]
fn test_error_conversion() {
    run(|ctx| {
        // Test conversion of different types of errors
        let cases = [
            (b"throw new TypeError('type error')" as &[u8], "type error"),
            (b"throw new ReferenceError('ref error')", "ref error"),
            (b"throw new SyntaxError('syntax error')", "syntax error"),
            (b"throw new Error('general error')", "general error"),
        ];

        for (code, expected_msg) in cases {
            let err = ctx.eval::<()>(Source::from_bytes(code)).unwrap_err();
            let message = thrown_error_message(ctx, &err)?;
            assert_eq!(message, expected_msg);
        }
        Ok(())
    });
}

#[test]
fn test_error_display() {
    run(|ctx| {
        // Test formatting of error messages
        let result = ctx.eval::<()>(Source::from_bytes(
            b"
            function foo() {
                throw new Error('test error');
            }
            foo();
        ",
        ));

        let err = result.unwrap_err();
        let message = thrown_error_message(ctx, &err)?;
        let stack = thrown_error_stack(ctx, &err)?;
        assert!(
            message.contains("test error"),
            "Error string should contain message"
        );
        assert!(
            stack.contains("foo"),
            "Error string should contain stack trace"
        );
        Ok(())
    });
}
