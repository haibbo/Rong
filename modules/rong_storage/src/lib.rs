//! # Rong Storage Module
//!
//! A synchronous key-value storage implementation based on redb.
//!
//! ## Features
//! - Synchronous API (no async overhead)
//! - Type-preserving JSON serialization
//! - Size limits and error handling
//! - Iterator support for key listing
//! - Thread-local global storage
//!
//! ## Supported Data Types
//! - **Strings**: Stored as JSON strings
//! - **Numbers**: i32, u32, f64 (JavaScript `number` type)
//! - **BigInts**: i64, u64 (JavaScript `bigint` type)
//! - **Booleans**: true/false
//! - **null**: JavaScript null
//! - **Objects**: Serialized via JSON.stringify
//! - **Arrays**: Serialized via JSON.stringify
//!
//! ## Limitations
//! - `undefined` values are rejected
//! - Extremely large unsigned values like `u64::MAX` may not round-trip perfectly
//!   due to JavaScript BigInt to native type conversion limitations in QuickJS
//! - Maximum key size: 1KB
//! - Maximum value size: 5MB
//! - Maximum total storage: 10MB

use redb::{Database, ReadableTable, TableDefinition};
use rong::{IntoJSIteratorExt, function::Optional, *};
use serde_json;
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;

mod storage;
use storage::*;

// Table definitions
const STORAGE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("storage");

// Default size limits
const REDB_INIT_OVERHEAD: usize = (1.5 * 1024.0 * 1024.0) as usize; // ~1.5MB redb database initialization overhead
const DEFAULT_MAX_TOTAL_SIZE: usize = 20 * 1024 * 1024; // 20MB total storage limit
const DEFAULT_MAX_USER_DATA_SIZE: usize = DEFAULT_MAX_TOTAL_SIZE + REDB_INIT_OVERHEAD;
const DEFAULT_MAX_KEY_SIZE: usize = 1024; // 1KB
const DEFAULT_MAX_VALUE_SIZE: usize = 5 * 1024 * 1024; // 5MB

// Thread-local storage database (single database per thread)
thread_local! {
    static STORAGE_DB: RefCell<Option<Rc<Database>>> = RefCell::new(None);
}

/// Set the storage database path and create the database
pub fn set_storage_path<P: AsRef<Path>>(path: P) -> Result<(), String> {
    let db_path = path.as_ref();

    // Create parent directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    // Create the database
    let db = Database::create(db_path)
        .map_err(|e| format!("Failed to open database at {:?}: {}", db_path, e))?;

    // Store in thread-local storage
    STORAGE_DB.with(|storage| {
        *storage.borrow_mut() = Some(Rc::new(db));
    });

    Ok(())
}

/// Get the thread-local database
fn get_storage_db() -> Option<Rc<Database>> {
    STORAGE_DB.with(|storage| storage.borrow().clone())
}

/// Close all open storages in the current thread
/// This is useful for cleanup in tests or when shutting down
pub fn close_all_storages() {
    STORAGE_DB.with(|storage| {
        storage.borrow_mut().take();
    });
}

// size is in bytes
#[derive(IntoJSObj)]
pub struct StorageInfo {
    #[rename = "currentSize"]
    current_size: u32,
    #[rename = "limitSize"]
    limit_size: u32,
    #[rename = "keyCount"]
    key_count: u32,
}

/// Storage list function that returns an iterator
fn storage_list(ctx: JSContext, prefix: Optional<String>) -> JSResult<JSValue> {
    let db = get_storage_db().ok_or_else(|| {
        RongJSError::TypeError("Storage not initialized. Call set_storage_path first.".to_string())
    })?;

    let read_txn = db
        .begin_read()
        .map_err(|e| RongJSError::TypeError(format!("Failed to begin read transaction: {}", e)))?;

    let table = read_txn
        .open_table(STORAGE_TABLE)
        .map_err(|e| RongJSError::TypeError(format!("Failed to open table: {}", e)))?;

    let iter = table
        .iter()
        .map_err(|e| RongJSError::TypeError(format!("Failed to create iterator: {}", e)))?;

    let mut keys = Vec::new();
    for item in iter {
        let (key, _) =
            item.map_err(|e| RongJSError::TypeError(format!("Failed to read item: {}", e)))?;
        let key_str = key.value().to_string();

        // Apply prefix filter if provided
        if let Some(ref prefix_str) = prefix.0 {
            if key_str.starts_with(prefix_str) {
                keys.push(key_str);
            }
        } else {
            keys.push(key_str);
        }
    }

    // Convert to JS iterator and then to JSValue
    let iter = keys.to_js_iter(&ctx)?;
    Ok(JSValue::from(&ctx, iter))
}

/// Storage info function
fn storage_info() -> JSResult<StorageInfo> {
    let db = get_storage_db().ok_or_else(|| {
        RongJSError::TypeError("Storage not initialized. Call set_storage_path first.".to_string())
    })?;

    let read_txn = db
        .begin_read()
        .map_err(|e| RongJSError::TypeError(format!("Failed to begin read transaction: {}", e)))?;

    let table = read_txn
        .open_table(STORAGE_TABLE)
        .map_err(|e| RongJSError::TypeError(format!("Failed to open table: {}", e)))?;

    let mut current_size = 0;
    let mut key_count = 0;
    let iter = table
        .iter()
        .map_err(|e| RongJSError::TypeError(format!("Failed to create iterator: {}", e)))?;

    for item in iter {
        let (key, value) =
            item.map_err(|e| RongJSError::TypeError(format!("Failed to read item: {}", e)))?;

        current_size += key.value().len() + value.value().len();
        key_count += 1;
    }

    Ok(StorageInfo {
        current_size: current_size as u32,
        limit_size: DEFAULT_MAX_USER_DATA_SIZE as u32,
        key_count: key_count as u32,
    })
}

/// Initialize the Storage module
pub fn init(ctx: &JSContext) -> JSResult<()> {
    // Create default storage database if not already created
    if get_storage_db().is_none() {
        // Use default path if no path was set
        let default_path = "storage.db";
        set_storage_path(default_path).map_err(|e| {
            RongJSError::TypeError(format!("Failed to initialize default storage: {}", e))
        })?;
    }

    let rong = ctx.rong();

    // Create storage object with methods
    let storage = JSObject::new(ctx);

    storage
        .set("set", JSFunc::new(ctx, storage_set)?.name("set")?)?
        .set("get", JSFunc::new(ctx, storage_get)?.name("get")?)?
        .set("delete", JSFunc::new(ctx, storage_delete)?.name("delete")?)?
        .set("clear", JSFunc::new(ctx, storage_clear)?.name("clear")?)?
        .set("list", JSFunc::new(ctx, storage_list)?.name("list")?)?
        .set("info", JSFunc::new(ctx, storage_info)?.name("info")?)?;

    // Set as Rong.storage
    rong.set("storage", storage)?;

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

            // Set storage path using workspace root
            let storage_path = format!("{}/target/test-tmp/test_storage.db", workspace_root);
            set_storage_path(&storage_path).unwrap();

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
}
