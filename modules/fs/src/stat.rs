use rusty_js::*;
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::time::SystemTime;
use tokio::fs;

#[js_export]
struct FileInfo {
    is_file: bool,
    is_directory: bool,
    is_symlink: bool,
    size: f64,
    modified: Option<SystemTime>,
    accessed: Option<SystemTime>,
    created: Option<SystemTime>,
    mode: Option<u32>,
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
    let metadata = fs::metadata(&path)
        .await
        .map_err(|e| RustyJSError::TypeError(format!("Failed to get file info: {}", e)))?;

    #[cfg(unix)]
    let mode = Some(metadata.mode());
    #[cfg(not(unix))]
    let mode = None;

    let info = FileInfo {
        is_file: metadata.is_file(),
        is_directory: metadata.is_dir(),
        is_symlink: metadata.is_symlink(),
        size: metadata.len() as f64,
        modified: metadata.modified().ok(),
        accessed: metadata.accessed().ok(),
        created: metadata.created().ok(),
        mode,
    };

    Ok(info)
}

async fn lstat(path: String) -> JSResult<FileInfo> {
    let metadata = fs::symlink_metadata(&path)
        .await
        .map_err(|e| RustyJSError::TypeError(format!("Failed to get file info: {}", e)))?;

    #[cfg(unix)]
    let mode = Some(metadata.mode());
    #[cfg(not(unix))]
    let mode = None;

    let info = FileInfo {
        is_file: metadata.is_file(),
        is_directory: metadata.is_dir(),
        is_symlink: metadata.is_symlink(),
        size: metadata.len() as f64,
        modified: metadata.modified().ok(),
        accessed: metadata.accessed().ok(),
        created: metadata.created().ok(),
        mode,
    };

    Ok(info)
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    let danity = ctx.dainty();

    ctx.register_class::<FileInfo>()?;

    let stat_fn = JSFunc::new(ctx, stat)?.name("stat")?;
    danity.set("stat", stat_fn)?;

    let lstat_fn = JSFunc::new(ctx, lstat)?.name("lstat")?;
    danity.set("lstat", lstat_fn)?;

    Ok(())
}
