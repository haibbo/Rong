use rustyjs_test::*;

use std::string::String;

#[test]
fn test_convert() {
    run(|ctx| {
        let jsvalue = JSValue::from(ctx, false);
        assert!(jsvalue.is_boolean());
        assert!(!jsvalue.try_into::<bool>().unwrap());

        let jsvalue = JSValue::from(ctx, i32::MIN);
        assert!(jsvalue.is_number());
        assert_eq!(i32::MIN, jsvalue.try_into().unwrap());

        let jsvalue = JSValue::from(ctx, u32::MAX);
        assert_eq!(u32::MAX, jsvalue.try_into().unwrap());

        let jsvalue = JSValue::from(ctx, i64::MIN);
        assert_eq!(i64::MIN, jsvalue.try_into().unwrap());

        let jsvalue = JSValue::from(ctx, u64::MAX);
        assert!(jsvalue.is_bigint());
        assert_eq!(u64::MAX, jsvalue.try_into().unwrap());

        let jsvalue = JSValue::from(ctx, f64::MIN);
        assert_eq!(f64::MIN, jsvalue.try_into().unwrap());

        let hello = "Hello";
        let jsvalue = JSValue::from(ctx, hello);
        assert!(jsvalue.is_string());
        let output: String = jsvalue.try_into().unwrap();
        assert_eq!(String::from(hello), output);

        let jsvalue = JSValue::undefined(ctx);
        assert!(jsvalue.is_undefined());
        let output: String = jsvalue.try_into().unwrap();
        assert_eq!(String::from("UNDEFINED"), output);

        // Test usize conversion
        let test_usize: usize = 42;
        let jsvalue = JSValue::from(ctx, test_usize);
        assert!(jsvalue.is_bigint());
        let output: usize = jsvalue.try_into().unwrap();
        assert_eq!(test_usize, output);

        // Test large usize conversion
        let large_usize: usize = usize::MAX;
        let jsvalue = JSValue::from(ctx, large_usize);
        assert!(jsvalue.is_bigint());
        let output: usize = jsvalue.try_into().unwrap();
        assert_eq!(large_usize, output);
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

#[test]
fn test_string_with_null() {
    run(|ctx| {
        // Test string containing null character
        let result: String = ctx
            .eval(Source::from_bytes(
                br#"
                "before" + String.fromCharCode(0) + "after"
                "#,
            ))
            .unwrap();

        // In JavaScript, the string is: "before\0after"
        // "before" (6 chars) + "\0" (1 char) + "after" (5 chars) = 12 total
        assert_eq!(result.len(), 12, "String length should be 12 (6 + 1 + 5)");

        // Verify the string contains the null character
        assert!(
            result.contains('\0'),
            "String should contain null character"
        );

        // Verify content before and after null character
        let parts: Vec<&str> = result.split('\0').collect();
        assert_eq!(parts[0], "before", "Content before null character");
        assert_eq!(parts[1], "after", "Content after null character");

        // Test empty string with only null character
        let result: String = ctx
            .eval(Source::from_bytes(
                br#"
                String.fromCharCode(0)
                "#,
            ))
            .unwrap();
        assert_eq!(
            result.len(),
            1,
            "Single null character string length should be 1"
        );
        assert_eq!(result.as_bytes(), &[0], "Should be a single null byte");

        // Test string with multiple null characters
        let result: String = ctx
            .eval(Source::from_bytes(
                br#"
                "a" + String.fromCharCode(0) + "b" + String.fromCharCode(0) + "c"
                "#,
            ))
            .unwrap();
        assert_eq!(
            result.len(),
            5,
            "String length should be 5 (a + \0 + b + \0 + c)"
        );
        let parts: Vec<&str> = result.split('\0').collect();
        assert_eq!(parts, &["a", "b", "c"], "Should split into three parts");
    });
}
