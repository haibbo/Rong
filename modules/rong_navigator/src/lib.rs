use rong::{JSContext, JSObject, JSResult};
use std::env;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    let navigator = JSObject::new(ctx);

    // Initialize user agent if not already set
    let ua = rong::get_user_agent();

    navigator.set("userAgent", ua.as_str())?;
    navigator.set("platform", env::consts::OS)?;
    navigator.set("arch", env::consts::ARCH)?;
    ctx.global().set("navigator", navigator)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong_test::*;

    #[test]
    fn test_user_agent() {
        run(|ctx| {
            // Test setting custom user agent
            rong::set_user_agent("CustomUA/1.0")
                .map_err(|e| rong::HostError::new(rong::error::E_INVALID_ARG, e))?;
            let custom_ua = rong::get_user_agent();
            assert_eq!(custom_ua, "CustomUA/1.0");

            init(ctx)?;

            // Verify navigator object reflects the change
            let navigator_ua: String = ctx.eval(Source::from_bytes(b"navigator.userAgent"))?;
            assert_eq!(navigator_ua, "CustomUA/1.0");

            Ok(())
        });
    }
}
