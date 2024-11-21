use crate::{impl_js_values, FromWithCtx};
use crate::{JSCtx, JSValueInner};

pub struct JSValue<'ctx>(pub(crate) JSValueInner<'ctx>);

impl_js_values!(bool, i32, u32, i64, u64, f64, String);

#[cfg(test)]
mod test {
    use crate::*;
    use std::string::String;

    #[test]
    fn test_value() {
        test_with(|ctx| {
            let jsvalue = JSValue::from_with_ctx(ctx, true);
            assert_eq!(true, jsvalue.try_into().unwrap());

            let jsvalue = JSValue::from_with_ctx(ctx, i32::MIN);
            assert_eq!(i32::MIN, jsvalue.try_into().unwrap());

            let jsvalue = JSValue::from_with_ctx(ctx, u32::MAX);
            assert_eq!(u32::MAX, jsvalue.try_into().unwrap());

            let jsvalue = JSValue::from_with_ctx(ctx, i64::MIN);
            assert_eq!(i64::MIN, jsvalue.try_into().unwrap());

            let jsvalue = JSValue::from_with_ctx(ctx, u64::MAX);
            assert_eq!(u64::MAX, jsvalue.try_into().unwrap());

            let jsvalue = JSValue::from_with_ctx(ctx, f64::MIN);
            assert_eq!(f64::MIN, jsvalue.try_into().unwrap());

            let str: String = String::from("hi");
            let jsvalue = JSValue::from_with_ctx(ctx, str.clone());
            assert_eq!(str, TryInto::<String>::try_into(jsvalue).unwrap());
        });
    }
}
