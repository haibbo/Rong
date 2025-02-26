use rusty_js::*;

mod base64;
mod text_decoder;
mod text_encoder;

pub use base64::{atob, btoa};
pub use text_decoder::TextDecoder;
pub use text_encoder::TextEncoder;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    text_encoder::init(ctx)?;
    text_decoder::init(ctx)?;

    let atob = JSFunc::new(ctx, atob)?;
    let btoa = JSFunc::new(ctx, btoa)?;
    ctx.global().set("atob", atob)?.set("btoa", btoa)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyjs_test::*;

    #[test]
    fn test_encoding() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx)?;
            console::init(&ctx, None)?;

            let passed = UnitJSRunner::load_script(&ctx, "encoding.js")
                .await?
                .run()
                .await?;
            assert!(passed);
            Ok(())
        });
    }
}
