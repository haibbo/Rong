mod helper;
use helper::*;

#[test]
fn test_throw_error() {
    run(|ctx| {
        let error = ctx.throw_syntax_error("Invalid syntax");
        assert_some!(error.is_exception());

        let error = ctx.throw_type_error("Invalid type");
        assert_some!(error.is_exception());

        let error = ctx.throw_reference_error("Undefined variable");
        assert_some!(error.is_exception());

        let error = ctx.eval::<()>("throw 'throw-error'").unwrap_err();
        let error = error.to_string();
        assert!(
            error.contains("throw-error"),
            "Expected error message to contain 'throw-error', but got: {}",
            error
        );
    });

    run(|ctx| {
        let e = Exception::from_message(ctx, "hi");
        assert_eq!(e.message().unwrap(), "hi");
    });
}
