use rong::*;
use std::cell::RefCell;

mod dir;
mod file;
mod misc;
mod read;
mod stat;
mod write;

/// File access guard trait for controlling file access permissions
pub trait FileAccessGuard: Send + Sync {
    /// Check if access to the given file path is allowed
    /// Returns Ok(()) if access is granted, Err with error message if denied
    fn check_access(&self, path: &str) -> JSResult<()>;
}

/// Default implementation that allows all file access
struct DefaultFileAccessGuard;

impl FileAccessGuard for DefaultFileAccessGuard {
    fn check_access(&self, _path: &str) -> JSResult<()> {
        Ok(()) // Allow all access by default
    }
}

// Thread-local storage for the file access guard
thread_local! {
    static FILE_ACCESS_GUARD: RefCell<Option<Box<dyn FileAccessGuard>>> = RefCell::new(None);
}

/// Set a custom file access guard
pub fn set_file_access_guard(guard: Box<dyn FileAccessGuard>) {
    FILE_ACCESS_GUARD.with(|g| {
        *g.borrow_mut() = Some(guard);
    });
}

/// Internal function to grant file access if allowed
fn grant_file_access(path: &str) -> JSResult<()> {
    FILE_ACCESS_GUARD.with(|g| {
        let guard_ref = g.borrow();
        let guard = guard_ref
            .as_ref()
            .map(|g| g.as_ref())
            .unwrap_or(&DefaultFileAccessGuard as &dyn FileAccessGuard);

        guard.check_access(path)
    })
}

pub fn init(ctx: &JSContext) -> JSResult<()> {
    // Ensure stream classes are registered for fs.readable support
    rong_stream::init(ctx)?;
    read::init(ctx)?;
    write::init(ctx)?;
    dir::init(ctx)?;
    stat::init(ctx)?;
    misc::init(ctx)?;
    file::init(ctx)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong_test::*;
    use std::env;

    #[test]
    fn test_filesystem() {
        async_run!(|ctx: JSContext| async move {
            rong_encoding::init(&ctx)?;
            rong_console::init(&ctx)?;
            rong_assert::init(&ctx)?;
            rong_abort::init(&ctx)?;
            rong_exception::init(&ctx)?;
            init(&ctx)?;

            // Get workspace root path
            let workspace_root = env::current_dir()
                .map_err(|e| RongJSError::TypeError(format!("Failed to get current dir: {}", e)))?
                .parent()
                .and_then(|p| p.parent()) // Go up two levels
                .ok_or_else(|| RongJSError::TypeError("Failed to get workspace root".into()))?
                .to_string_lossy()
                .into_owned();

            // Inject workspace root into JavaScript environment
            ctx.global().set("WORKSPACE_ROOT", workspace_root)?;

            let passed_fs = UnitJSRunner::load_script(&ctx, "filesystem.js")
                .await?
                .run()
                .await?;
            assert!(passed_fs);

            Ok(())
        });
    }

    // Note: FsFile.readable tests live in tests/unit/filesystem.js
}
