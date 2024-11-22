use crate::{impl_js_values, FromHost, IntoHost};
use crate::{JSCtx, JSValueInner};

#[derive(Clone)]
pub struct JSValue<'ctx>(pub(crate) JSValueInner<'ctx>);

impl<'ctx> From<JSValueInner<'ctx>> for JSValue<'ctx> {
    fn from(v: JSValueInner<'ctx>) -> Self {
        Self(v)
    }
}

impl<'ctx> FromHost<'ctx, &str> for JSValue<'ctx> {
    type Context = JSCtx;
    fn from_host(ctx: &'ctx Self::Context, value: &str) -> Self {
        JSValueInner::from_host(&ctx.0, value).into()
    }
}

impl<'ctx> IntoHost<String> for JSValue<'ctx> {
    fn into_host(self) -> Option<String> {
        self.0.into_host()
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
            let jsvalue = JSValue::from_host(ctx, true);
            assert_eq!(true, jsvalue.into_host().unwrap());

            let jsvalue = JSValue::from_host(ctx, i32::MIN);
            assert_eq!(i32::MIN, jsvalue.into_host().unwrap());

            let jsvalue = JSValue::from_host(ctx, u32::MAX);
            assert_eq!(u32::MAX, jsvalue.into_host().unwrap());

            let jsvalue = JSValue::from_host(ctx, i64::MIN);
            assert_eq!(i64::MIN, jsvalue.into_host().unwrap());

            let jsvalue = JSValue::from_host(ctx, u64::MAX);
            assert_eq!(u64::MAX, jsvalue.into_host().unwrap());

            let jsvalue = JSValue::from_host(ctx, f64::MIN);
            assert_eq!(f64::MIN, jsvalue.into_host().unwrap());

            let hello = "Hello";
            let jsvalue = JSValue::from_host(ctx, hello.as_ref());
            assert_eq!(
                String::from(hello),
                IntoHost::<String>::into_host(jsvalue).unwrap()
            );
        });
    }
}
