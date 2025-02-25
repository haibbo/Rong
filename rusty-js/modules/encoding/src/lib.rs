use rusty_js::*;

mod text_decoder;
mod text_encoder;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    text_encoder::init(ctx)?;
    text_decoder::init(ctx)?;

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
