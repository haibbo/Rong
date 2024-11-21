use crate::{impl_js_values, FromWithCtx};
use crate::{JSCtx, JSValueInner};

pub struct JSValue<'ctx>(pub(crate) JSValueInner<'ctx>);

impl<'ctx> From<JSValueInner<'ctx>> for JSValue<'ctx> {
    fn from(v: JSValueInner<'ctx>) -> Self {
        Self(v)
    }
}

impl<'ctx> FromWithCtx<'ctx, &str> for JSValue<'ctx> {
    type Context = JSCtx;
    fn from_with_ctx(ctx: &'ctx Self::Context, value: &str) -> Self {
        JSValueInner::from_with_ctx(&ctx.0, value).into()
    }
}

impl<'ctx> TryInto<String> for JSValue<'ctx> {
    type Error = ();
    fn try_into(self) -> Result<String, Self::Error> {
        self.0.try_into()
    }
}

impl_js_values!(bool, i32, u32, i64, u64, f64);

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

            let hello = "Hello";
            let jsvalue = JSValue::from_with_ctx(ctx, hello.as_ref());
            assert_eq!(
                String::from(hello),
                TryInto::<String>::try_into(jsvalue).unwrap()
            );
        });
    }
}
