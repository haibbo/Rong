use rustyjs_test::*;

#[test]
fn test_throw_error() {
    run(|ctx| {
        let error = ctx.throw_syntax_error("Invalid syntax");
        assert_some!(error.is_exception());

        let error = ctx.throw_type_error("Invalid type");
        assert_some!(error.is_exception());

        let error = ctx.throw_reference_error("Undefined variable");
        assert_some!(error.is_exception());

        let error = ctx
            .eval::<()>(Source::from_bytes(b"throw new Error('throw-error')"))
            .unwrap_err();
        let error = error.to_string();
        assert!(
            error.contains("throw-error"),
            "Expected error message to contain 'throw-error', but got: {}",
            error
        );
    });

    run(|ctx| {
        let e = ctx.new_js_error("hi");
        assert_eq!(e.message().unwrap(), "hi");
    });
}

#[test]
fn test_error_stack() {
    run(|ctx| {
        // test syntax error
        let result = ctx.eval::<()>(Source::from_bytes(b"function test() { a b c }"));
        let RustyJSError::Exception(error) = result.unwrap_err() else {
            panic!("Expected JSError");
        };
        assert!(error.message.is_some(), "Should have error message");
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
        let RustyJSError::Exception(error) = result.unwrap_err() else {
            panic!("Expected JSError");
        };
        assert!(
            error.message.unwrap().contains("bar"),
            "Error message should mention undefined variable"
        );
        assert!(
            error.stack.unwrap().contains("foo"),
            "Stack trace should contain function name"
        );

        // test type error
        let result = ctx.eval::<()>(Source::from_bytes(
            b"
            let obj = null;
            obj.property;  // TypeError: Cannot read property of null
        ",
        ));
        let RustyJSError::Exception(error) = result.unwrap_err() else {
            panic!("Expected JSError");
        };
        assert!(
            error.message.unwrap().contains("null"),
            "Error message should mention null"
        );
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

        let RustyJSError::Exception(error) = result.unwrap_err() else {
            panic!("Expected JSError");
        };
        assert_eq!(error.message.unwrap(), "Custom error message");
        let stack = error.stack.unwrap();
        assert!(
            stack.contains("throwCustomError"),
            "Stack should contain throwCustomError"
        );
        assert!(stack.contains("caller"), "Stack should contain caller");
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
            let RustyJSError::Exception(error) =
                ctx.eval::<()>(Source::from_bytes(code)).unwrap_err()
            else {
                panic!("Expected JSError");
            };
            assert_eq!(error.message.unwrap(), expected_msg);
        }
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

        let error = result.unwrap_err();
        let error_str = error.to_string();
        assert!(
            error_str.contains("test error"),
            "Error string should contain message"
        );
        assert!(
            error_str.contains("foo"),
            "Error string should contain stack trace"
        );
    });
}
