use rustyjs_test::*;

use std::string::String;

#[test]
fn test_convert() {
    run(|ctx| {
        let jsvalue = JSValue::from(ctx, false);
        assert_some!(jsvalue.is_boolean());
        assert!(!jsvalue.try_into::<bool>().unwrap());

        let jsvalue = JSValue::from(ctx, i32::MIN);
        assert_some!(jsvalue.is_number());
        assert_eq!(i32::MIN, jsvalue.try_into().unwrap());

        let jsvalue = JSValue::from(ctx, u32::MAX);
        assert_eq!(u32::MAX, jsvalue.try_into().unwrap());

        let jsvalue = JSValue::from(ctx, i64::MIN);
        assert_eq!(i64::MIN, jsvalue.try_into().unwrap());

        let jsvalue = JSValue::from(ctx, u64::MAX);
        assert_eq!(u64::MAX, jsvalue.try_into().unwrap());

        let jsvalue = JSValue::from(ctx, f64::MIN);
        assert_eq!(f64::MIN, jsvalue.try_into().unwrap());

        let hello = "Hello";
        let jsvalue = JSValue::from(ctx, hello);
        assert_some!(jsvalue.is_string());
        let output: String = jsvalue.try_into().unwrap();
        assert_eq!(String::from(hello), output);

        let jsvalue = JSValue::undefined(ctx);
        assert_some!(jsvalue.is_undefined());
        let output: String = jsvalue.try_into().unwrap();
        assert_eq!(String::from("UNDEFINED"), output);
    });
}

#[test]
fn test_display() {
    run(|ctx| {
        // Test undefined
        let jsvalue = JSValue::undefined(ctx);
        assert_eq!(format!("{}", jsvalue), "undefined");

        // Test boolean
        let jsvalue = JSValue::from(ctx, true);
        assert_eq!(format!("{}", jsvalue), "true");

        // Test number
        let jsvalue = JSValue::from(ctx, 42);
        assert_eq!(format!("{}", jsvalue), "42");

        // Test string
        let jsvalue = JSValue::from(ctx, "hello");
        assert_eq!(format!("{}", jsvalue), "hello");

        // Test object
        let code = "({foo: 'bar'})";
        let jsvalue: JSValue = ctx.eval(Source::from_bytes(code)).unwrap();
        assert_eq!(format!("{}", jsvalue), "object");

        // Test array
        let code = "[1, 2, 3]";
        let jsvalue: JSValue = ctx.eval(Source::from_bytes(code)).unwrap();
        assert_eq!(format!("{}", jsvalue), "array");

        // Test function
        let code = "(function name() {})";
        let jsvalue: JSValue = ctx.eval(Source::from_bytes(code)).unwrap();
        assert_eq!(format!("{}", jsvalue), "function");

        // Test promise
        let code = "new Promise(() => {})";
        let jsvalue: JSValue = ctx.eval(Source::from_bytes(code)).unwrap();
        assert_eq!(format!("{}", jsvalue), "promise");

        // Test error
        let code = "new Error('test error')";
        let jsvalue: JSValue = ctx.eval(Source::from_bytes(code)).unwrap();
        assert_eq!(format!("{}", jsvalue), "error");

        // Test null
        let code = "null";
        let jsvalue: JSValue = ctx.eval(Source::from_bytes(code)).unwrap();
        assert_eq!(format!("{}", jsvalue), "null");
    });
}
