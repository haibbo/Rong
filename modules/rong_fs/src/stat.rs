use crate::grant_file_access;
use rong::*;
use std::{fs, time::SystemTime};
use tokio::fs as tokio_fs;

#[js_export]
pub(crate) struct FileInfo {
    is_file: bool,
    is_directory: bool,
    is_symlink: bool,
    size: f64,
    modified: Option<SystemTime>,
    accessed: Option<SystemTime>,
    created: Option<SystemTime>,
    mode: Option<u32>,
}

impl FileInfo {
    /// Create a new FileInfo from fs::Metadata
    pub(crate) fn from_metadata(metadata: fs::Metadata) -> Self {
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::MetadataExt;
            Some(metadata.mode())
        };
        #[cfg(not(unix))]
        let mode = None;

        Self {
            is_file: metadata.is_file(),
            is_directory: metadata.is_dir(),
            is_symlink: metadata.is_symlink(),
            size: metadata.len() as f64,
            modified: metadata.modified().ok(),
            accessed: metadata.accessed().ok(),
            created: metadata.created().ok(),
            mode,
        }
    }
}

#[js_class]
impl FileInfo {
    #[js_method(constructor)]
    fn new() -> Self {
        Self {
            is_file: false,
            is_directory: false,
            is_symlink: false,
            size: 0.0,
            modified: None,
            accessed: None,
            created: None,
            mode: None,
        }
    }

    #[js_method(getter, rename = "isFile")]
    fn is_file(&self) -> bool {
        self.is_file
    }

    #[js_method(getter, rename = "isDirectory")]
    fn is_directory(&self) -> bool {
        self.is_directory
    }

    #[js_method(getter, rename = "isSymlink")]
    fn is_symlink(&self) -> bool {
        self.is_symlink
    }

    #[js_method(getter)]
    fn size(&self) -> f64 {
        self.size
    }

    #[js_method(getter)]
    fn modified(&self) -> Option<f64> {
        self.modified.map(|t| {
            t.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as f64
        })
    }

    #[js_method(getter)]
    fn accessed(&self) -> Option<f64> {
        self.accessed.map(|t| {
            t.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as f64
        })
    }

    #[js_method(getter)]
    fn created(&self) -> Option<f64> {
        self.created.map(|t| {
            t.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as f64
        })
    }

    #[js_method(getter)]
    fn mode(&self) -> Option<u32> {
        self.mode
    }
}

async fn stat(path: String) -> JSResult<FileInfo> {
    let resolved = grant_file_access(&path)?;
    tokio_fs::metadata(&resolved)
        .await
        .map(FileInfo::from_metadata)
        .map_err(|e| HostError::new("FS_IO", format!("Failed to get file info: {}", e)).into())
}

async fn lstat(path: String) -> JSResult<FileInfo> {
    let resolved = grant_file_access(&path)?;
    tokio_fs::symlink_metadata(&resolved)
        .await
        .map(FileInfo::from_metadata)
        .map_err(|e| HostError::new("FS_IO", format!("Failed to get file info: {}", e)).into())
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    let rong = ctx.rong();

    ctx.register_class::<FileInfo>()?;

    let stat_fn = JSFunc::new(ctx, stat)?.name("stat")?;
    rong.set("stat", stat_fn)?;

    let lstat_fn = JSFunc::new(ctx, lstat)?.name("lstat")?;
    rong.set("lstat", lstat_fn)?;

    Ok(())
}
