mod blob;
mod file;

pub use blob::Blob;
pub use file::File;

use rusty_js::*;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<Blob>()?;
    ctx.register_class::<File>()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyjs_test::*;

    #[test]
    fn test_path() {
        async_run!(|ctx: JSContext| async move {
            encoding::init(&ctx)?;
            console::init(&ctx)?;
            assert::init(&ctx)?;
            init(&ctx)?;

            let passed = UnitJSRunner::load_script(&ctx, "buffer.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }
}
