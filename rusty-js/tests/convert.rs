mod helper;
use helper::*;

use std::string::String;

#[test]
fn test_convert() {
    run(|ctx| {
        let jsvalue = JSValue::from(ctx, false);
        assert_some!(jsvalue.is_boolean());
        assert!(!bool::from_js(jsvalue).unwrap());

        let jsvalue = JSValue::from(ctx, i32::MIN);
        assert_some!(jsvalue.is_number());
        assert_eq!(i32::MIN, jsvalue.js_into().unwrap());

        let jsvalue = JSValue::from(ctx, u32::MAX);
        assert_eq!(u32::MAX, jsvalue.js_into().unwrap());

        let jsvalue = JSValue::from(ctx, i64::MIN);
        assert_eq!(i64::MIN, jsvalue.js_into().unwrap());

        let jsvalue = JSValue::from(ctx, u64::MAX);
        assert_eq!(u64::MAX, jsvalue.js_into().unwrap());

        let jsvalue = JSValue::from(ctx, f64::MIN);
        assert_eq!(f64::MIN, f64::from_js(jsvalue).unwrap());

        let hello = "Hello";
        let jsvalue = JSValue::from(ctx, hello);
        assert_some!(jsvalue.is_string());
        let output: String = jsvalue.js_into().unwrap();
        assert_eq!(String::from(hello), output);

        let jsvalue = JSValue::undefined(ctx);
        assert_some!(jsvalue.is_undefined());
    });
}
