use crate::engine::JSValueInner;
use crate::{IntoHost, JSCtxExt, JSCtxInner, JSRuntime};
use anyhow;

#[derive(Clone)]
pub struct JSCtx(pub(crate) JSCtxInner);

impl JSCtx {
    pub fn new(rt: &JSRuntime) -> Result<Self, String> {
        JSCtxInner::new(&rt.inner).map(|inner| JSCtx(inner))
    }
}

impl<'ctx> JSCtxExt<'ctx> for JSCtx {
    type Value = JSValueInner<'ctx>;
    fn eval<S, T>(&'ctx self, source: S) -> anyhow::Result<T>
    where
        S: AsRef<str>,
        Self::Value: IntoHost<T>,
    {
        self.0.eval(source)
    }
}

#[cfg(test)]
mod test {
    use crate::*;
    use std::string::String;

    #[test]
    fn test_eval() {
        test_with(|ctx| {
            let result: i32 = ctx.eval("Math.sqrt(16)").unwrap();
            assert_eq!(result, 4);

            let result: String = ctx.eval("'hi'").unwrap(); // don't forget '',
            assert_eq!(result, String::from("hi"));

            let err = ctx.eval::<_, String>(r#"throw "hix""#).unwrap_err();
            assert_eq!(err.to_string(), String::from("hix"));

            let err = ctx
                .eval::<_, String>(r#"throw new Error("This is error")"#)
                .unwrap_err();
            assert!(err.to_string().contains("This is error"));
        });
    }
}
