use rusty_js::*;
use tokio::fs;

mod dir;
mod read;
mod stat;
mod write;

async fn rename(from: String, to: String) -> JSResult<()> {
    fs::rename(&from, &to)
        .await
        .map_err(|e| RustyJSError::TypeError(format!("Failed to rename file: {}", e)))
}

async fn real_path(path: String) -> JSResult<String> {
    fs::canonicalize(&path)
        .await
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| RustyJSError::TypeError(format!("Failed to resolve real path: {}", e)))
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    let danity = ctx.dainty();

    let rename_fn = JSFunc::new(ctx, rename)?.name("rename")?;
    danity.set("rename", rename_fn)?;

    let real_path_fn = JSFunc::new(ctx, real_path)?.name("realPath")?;
    danity.set("realPath", real_path_fn)?;

    read::init(ctx)?;
    write::init(ctx)?;
    dir::init(ctx)?;
    stat::init(ctx)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyjs_test::*;
    use std::env;

    #[test]
    fn test_filesystem() {
        async_run!(|ctx: JSContext| async move {
            encoding::init(&ctx)?;
            console::init(&ctx)?;
            assert::init(&ctx)?;
            abort::init(&ctx)?;
            dom_exception::init(&ctx)?;
            init(&ctx)?;

            // Get workspace root path
            let workspace_root = env::current_dir()
                .map_err(|e| RustyJSError::TypeError(format!("Failed to get current dir: {}", e)))?
                .parent()
                .ok_or_else(|| RustyJSError::TypeError("Failed to get workspace root".into()))?
                .to_string_lossy()
                .into_owned();

            // Inject workspace root into JavaScript environment
            ctx.global().set("WORKSPACE_ROOT", workspace_root)?;

            let passed = UnitJSRunner::load_script(&ctx, "filesystem.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }
}
