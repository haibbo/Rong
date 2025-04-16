mod blob;
mod file;

pub use blob::Blob;
pub use file::File;

use rong::*;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<Blob>()?;
    ctx.register_class::<File>()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong_test::*;

    #[test]
    fn test_path() {
        async_run!(|ctx: JSContext| async move {
            rong_encoding::init(&ctx)?;
            rong_console::init(&ctx)?;
            rong_assert::init(&ctx)?;
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
