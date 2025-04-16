use rong::{JSContext, JSObject, JSResult};
use std::env;
use std::sync::OnceLock;

static USER_AGENT: OnceLock<String> = OnceLock::new();

/// Sets a custom user agent string for the navigator.
///
/// This function must be called before initializing the navigator with `init()`.
/// If not called, the default user agent will be "RongJS/{version}".
pub fn set_user_agent(ua: &str) {
    let _ = USER_AGENT.set(ua.to_string());
}

/// Gets the current user agent string.
pub fn get_user_agent() -> &'static str {
    USER_AGENT.get_or_init(|| format!("RongJS/{}", env!("CARGO_PKG_VERSION")))
}

pub fn init(ctx: &JSContext) -> JSResult<()> {
    let navigator = JSObject::new(ctx);

    // Initialize user agent if not already set
    let ua = get_user_agent();

    navigator.set("userAgent", ua)?;
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
            set_user_agent("CustomUA/1.0");
            let custom_ua = get_user_agent();
            assert_eq!(custom_ua, "CustomUA/1.0");

            init(ctx)?;

            // Verify navigator object reflects the change
            let navigator_ua: String = ctx.eval(Source::from_bytes(b"navigator.userAgent"))?;
            assert_eq!(navigator_ua, "CustomUA/1.0");

            Ok(())
        });
    }
}
