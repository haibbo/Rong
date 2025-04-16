//! # URL Module
mod url;
mod url_search_params;

use rong_js::*;
pub use url::URL;
pub use url_search_params::URLSearchParams;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<URLSearchParams>()?;
    ctx.register_class::<URL>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong_test::*;

    #[test]
    fn test_url() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx)?;
            assert::init(&ctx)?;
            console::init(&ctx)?;

            let passed = UnitJSRunner::load_script(&ctx, "url.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }
}
