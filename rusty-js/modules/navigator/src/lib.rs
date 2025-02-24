use rusty_js::{JSContext, JSObject, JSResult};
use std::env;

fn get_user_agent() -> &'static str {
    concat!("RustyJS", env!("CARGO_PKG_VERSION"))
}

pub fn init(ctx: &JSContext, ua: Option<&str>) -> JSResult<()> {
    let navigator = JSObject::new(ctx);

    navigator.set("userAgent", ua.unwrap_or(get_user_agent()))?;
    navigator.set("platform", env::consts::OS)?;
    navigator.set("arch", env::consts::ARCH)?;
    ctx.global().set("navigator", navigator)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyjs_test::*;

    #[test]
    fn test_user_agent() {
        run(|ctx| {
            init(ctx, None).unwrap();
            let ua: String = ctx.eval(Source::from_bytes(b"navigator.userAgent"))?;
            assert!(ua.contains("RustyJS"));
            Ok(())
        });
    }
}
