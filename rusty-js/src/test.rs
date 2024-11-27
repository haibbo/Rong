#[cfg(test)]
mod tests {

    use crate::*;
    use std::string::String;

    #[test]
    fn convert() {
        test_with(|ctx| {
            let jsvalue = JSValue::from_rust(ctx, false);
            assert!(jsvalue.is_boolean());
            assert_eq!(false, jsvalue.into_rust().unwrap());

            let jsvalue = JSValue::from_rust(ctx, i32::MIN);
            assert!(jsvalue.is_number());
            assert_eq!(i32::MIN, jsvalue.into_rust().unwrap());

            let jsvalue = JSValue::from_rust(ctx, u32::MAX);
            assert_eq!(u32::MAX, jsvalue.into_rust().unwrap());

            let jsvalue = JSValue::from_rust(ctx, i64::MIN);
            assert_eq!(i64::MIN, jsvalue.into_rust().unwrap());

            let jsvalue = JSValue::from_rust(ctx, u64::MAX);
            assert_eq!(u64::MAX, jsvalue.into_rust().unwrap());

            let jsvalue = JSValue::from_rust(ctx, f64::MIN);
            assert_eq!(f64::MIN, jsvalue.into_rust().unwrap());

            let hello = "Hello";
            let jsvalue = JSValue::from_rust(ctx, hello.as_ref());
            assert!(jsvalue.is_string());
            assert_eq!(
                String::from(hello),
                JSValueInto::<String>::into_rust(jsvalue).unwrap()
            );
        });
    }
}
