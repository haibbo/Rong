mod helper;
use helper::*;

use std::string::String;

#[test]
fn test_convert() {
    run(|ctx| {
        let jsvalue = JSValue::from(ctx, false);
        assert!(jsvalue.is_boolean());
        assert_eq!(false, jsvalue.try_into().unwrap());

        let jsvalue = JSValue::from(ctx, i32::MIN);
        assert!(jsvalue.is_number());
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
        let jsvalue = JSValue::from(ctx, hello.as_ref());
        assert!(jsvalue.is_string());
        let output: String = jsvalue.try_into().unwrap();
        assert_eq!(String::from(hello), output);
    });
}
