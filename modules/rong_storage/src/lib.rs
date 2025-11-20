//! # Rong Storage Module
//!
//! Asynchronous key-value storage backed by `redb`, can manage their own database
//! files explicitly.
//!
//! ## Features
//! - Promise-based async API
//! - Type-preserving JSON serialization
//! - Configurable storage location per instance via constructor path argument
//! - Iterator support for key listing
//! - Automatic database creation when the file does not exist
//! - Optional per-instance limits (key/value/data size caps)
//!
//! ## Supported Data Types
//! - **Strings**, **Numbers**, **BigInts**, **Booleans**, **null**
//! - **Objects** and **Arrays** serialized via `JSON.stringify`
//! - **Date** objects preserved via a lightweight metadata envelope
//!
//! ## Limitations
//! - `undefined` values are rejected
//! - Extremely large unsigned values like `u64::MAX` may not round-trip perfectly due to
//!   JavaScript BigInt to native type conversion
//! - Maximum key size: 1KB
//! - Maximum value size: 5MB
//! - Maximum total storage (including redb overhead): ~21.5MB

use redb::TableDefinition;
use rong::{function::Optional, *};
use std::path::PathBuf;

mod storage;
pub use storage::*;

// Table definition shared by all storage instances
pub(crate) const STORAGE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("storage");

// Default size limits
const REDB_INIT_OVERHEAD: usize = (1.5 * 1024.0 * 1024.0) as usize; // ~1.5MB redb initialization overhead
pub(crate) const DEFAULT_MAX_TOTAL_SIZE: usize = 20 * 1024 * 1024; // 20MB user data limit
pub(crate) const DEFAULT_MAX_USER_DATA_SIZE: usize = DEFAULT_MAX_TOTAL_SIZE + REDB_INIT_OVERHEAD;
pub(crate) const DEFAULT_MAX_KEY_SIZE: usize = 1024; // 1KB
pub(crate) const DEFAULT_MAX_VALUE_SIZE: usize = 5 * 1024 * 1024; // 5MB

// size is in bytes
#[derive(IntoJSObj)]
pub struct StorageInfo {
    #[rename = "currentSize"]
    pub(crate) current_size: u32,
    #[rename = "limitSize"]
    pub(crate) limit_size: u32,
    #[rename = "keyCount"]
    pub(crate) key_count: u32,
}

/// Open a new storage instance at the provided path.
async fn storage_open(
    ctx: JSContext,
    path: String,
    options: Optional<StorageOptionsInput>,
) -> JSResult<JSObject> {
    let opts = options.0.map(StorageOptions::from).unwrap_or_default();
    let storage = Storage::open_with_options(PathBuf::from(path), opts)?;
    Ok(Class::get::<Storage>(&ctx)?.instance(storage))
}

/// Initialize the Storage module
pub fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<Storage>()?;

    let constructor = Class::get::<Storage>(ctx)?;
    let rong = ctx.rong();

    // Expose the class as Rong.Storage
    rong.set("Storage", constructor.clone())?;

    // Provide Rong.storage.open(path) helper for ergonomic access
    let storage_ns = JSObject::new(ctx);
    storage_ns.set("open", JSFunc::new(ctx, storage_open)?.name("open")?)?;
    rong.set("storage", storage_ns)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong_test::*;
    use std::env;

    #[test]
    fn test_storage() {
        async_run!(|ctx: JSContext| async move {
            // Get workspace root dynamically
            let workspace_root = env::current_dir()
                .map_err(|e| RongJSError::TypeError(format!("Failed to get current dir: {}", e)))?
                .parent()
                .and_then(|p| p.parent()) // Go up two levels
                .ok_or_else(|| RongJSError::TypeError("Failed to get workspace root".into()))?
                .to_string_lossy()
                .into_owned();

            // Provide test storage path to JS
            let storage_path = format!("{}/target/test-tmp/test_storage.db", workspace_root);
            ctx.global().set("TEST_STORAGE_DB_PATH", storage_path)?;

            rong_assert::init(&ctx)?;
            rong_console::init(&ctx)?;
            init(&ctx)?;

            let passed = UnitJSRunner::load_script(&ctx, "storage.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }

    #[test]
    fn rust_open_api_provides_handles() {
        let workspace_root = env::current_dir()
            .expect("cwd")
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root")
            .to_path_buf();

        let default_path = workspace_root.join("target/test-tmp/rust_storage_default.db");
        Storage::open(default_path).expect("default open should succeed");

        let custom_path = workspace_root.join("target/test-tmp/rust_storage_custom.db");
        let mut options = StorageOptions::default();
        options.max_key_size = Some(16);
        Storage::open_with_options(custom_path, options).expect("custom open should succeed");
    }

    #[test]
    fn close_allows_reopen_same_path() {
        let workspace_root = env::current_dir()
            .expect("cwd")
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root")
            .to_path_buf();

        let path = workspace_root.join("target/test-tmp/rust_storage_reopen.db");

        // First open and immediately close the database.
        let storage = Storage::open(&path).expect("initial open should succeed");
        storage.close();

        // After close, reopening the same path in the same process should succeed
        // without hitting redb's "Database already open. Cannot acquire lock" error.
        let _storage2 =
            Storage::open(&path).expect("reopen after close should succeed without lock error");
    }
}
