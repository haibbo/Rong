use rusty_js::*;

mod read;
mod write;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    read::init(ctx)?;
    write::init(ctx)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyjs_test::*;

    #[test]
    fn test_filesystem() {
        async_run!(|ctx: JSContext| async move {
            encoding::init(&ctx)?;
            console::init(&ctx)?;
            assert::init(&ctx)?;
            abort::init(&ctx)?;
            init(&ctx)?;

            let passed = UnitJSRunner::load_script(&ctx, "filesystem.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }
}
