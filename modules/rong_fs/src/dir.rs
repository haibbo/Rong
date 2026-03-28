use crate::grant_file_access;
use futures::Stream;
use rong::{function::Optional, *};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::fs;

#[js_export]
pub struct DirEntry {
    name: String,
    file_type: bool,
    is_symlink: bool,
}

#[js_class]
impl DirEntry {
    #[js_method(constructor)]
    fn new(_name: String, _file_type: bool, _is_symlink: bool) -> JSResult<Self> {
        rong::illegal_constructor("DirEntry cannot be constructed directly. Use Rong.readDir().")
    }

    #[js_method(getter)]
    fn name(&self) -> String {
        self.name.clone()
    }

    #[js_method(getter, rename = "isFile")]
    fn is_file(&self) -> bool {
        !self.file_type
    }

    #[js_method(getter, rename = "isDirectory")]
    fn is_dir(&self) -> bool {
        self.file_type
    }

    #[js_method(getter, rename = "isSymlink")]
    fn is_symlink(&self) -> bool {
        self.is_symlink
    }

    #[js_method(gc_mark)]
    fn gc_mark_with<F>(&self, _mark_fn: F)
    where
        F: FnMut(&JSValue),
    {
    }
}

#[derive(FromJSObj, Default)]
struct MkdirOptions {
    // If true, parent folders will be created if they don't exist
    recursive: Option<bool>,
    // Permissions to set on the created directory
    #[cfg(unix)]
    mode: Option<u32>,
}

async fn mkdir(path: String, option: Optional<MkdirOptions>) -> JSResult<()> {
    let resolved = grant_file_access(&path)?;
    let options = option.0.unwrap_or_default();

    // Check if directory exists first
    if let Ok(metadata) = fs::metadata(&resolved).await
        && metadata.is_dir()
    {
        return Ok(()); // Directory already exists, return success
    }

    let result = if options.recursive.unwrap_or(false) {
        fs::create_dir_all(&resolved).await
    } else {
        fs::create_dir(&resolved).await
    };

    result.map_err(|e| HostError::new("FS_IO", format!("Failed to create directory: {}", e)))?;

    // Set mode if specified (Unix-like systems only)
    #[cfg(unix)]
    if let Some(mode) = options.mode {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(mode);
        tokio::fs::set_permissions(&resolved, permissions)
            .await
            .map_err(|e| {
                HostError::new(
                    "FS_IO",
                    format!("Failed to set directory permissions: {}", e),
                )
            })?;
    }

    Ok(())
}

type FileTypeFuture =
    Pin<Box<dyn futures::Future<Output = Result<std::fs::FileType, std::io::Error>> + Send>>;

pub struct DirEntryStream {
    entries: fs::ReadDir,
    current_entry: Option<fs::DirEntry>,
    current_file_type_fut: Option<FileTypeFuture>,
}

impl DirEntryStream {
    pub fn new(entries: fs::ReadDir) -> Self {
        Self {
            entries,
            current_entry: None,
            current_file_type_fut: None,
        }
    }
}

impl Stream for DirEntryStream {
    type Item = Result<DirEntry, RongJSError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();

        // First, try to get the file type if we have a pending future
        if let Some(file_type_fut) = this.current_file_type_fut.as_mut() {
            match file_type_fut.as_mut().poll(cx) {
                Poll::Ready(Ok(file_type)) => {
                    this.current_file_type_fut.take();
                    if let Some(entry) = this.current_entry.take() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        return Poll::Ready(Some(Ok(DirEntry {
                            name,
                            file_type: file_type.is_dir(),
                            is_symlink: file_type.is_symlink(),
                        })));
                    }
                }
                Poll::Ready(Err(e)) => {
                    this.current_file_type_fut.take();
                    this.current_entry.take();
                    return Poll::Ready(Some(Err(HostError::new(
                        "FS_IO",
                        format!("Failed to get file type: {}", e),
                    )
                    .into())));
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }

        // If we have neither, try to get the next entry
        match this.entries.poll_next_entry(cx) {
            Poll::Ready(Ok(Some(entry))) => {
                let path = entry.path();
                let file_type_fut = Box::pin(async move {
                    let metadata = fs::symlink_metadata(&path).await?;
                    Ok(metadata.file_type())
                });
                this.current_entry = Some(entry);
                this.current_file_type_fut = Some(file_type_fut);
                cx.waker().wake_by_ref(); // Wake up the task to poll again
                Poll::Pending
            }
            Poll::Ready(Ok(None)) => Poll::Ready(None),
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(HostError::new(
                "FS_IO",
                format!("Failed to read directory entry: {}", e),
            )
            .into()))),
            Poll::Pending => Poll::Pending,
        }
    }
}

async fn readdir(ctx: JSContext, path: String) -> JSResult<JSObject> {
    let resolved = grant_file_access(&path)?;
    let entries = fs::read_dir(&resolved)
        .await
        .map_err(|e| HostError::new("FS_IO", format!("Failed to read directory: {}", e)))?;

    let stream = DirEntryStream::new(entries);
    stream.to_js_async_iter(&ctx)
}

#[derive(FromJSObj, Default)]
struct RemoveOptions {
    // If set to true, path will be removed even if it's a non-empty directory.
    recursive: bool,
}

async fn remove(path: String, option: Optional<RemoveOptions>) -> JSResult<()> {
    let resolved = grant_file_access(&path)?;
    let options = option.0.unwrap_or_default();

    // Check if path exists and get its type
    match fs::metadata(&resolved).await {
        Ok(metadata) => {
            if metadata.is_file() || metadata.is_symlink() {
                fs::remove_file(&resolved).await.map_err(|e| {
                    HostError::new("FS_IO", format!("Failed to remove file: {}", e)).into()
                })
            } else if metadata.is_dir() {
                if options.recursive {
                    fs::remove_dir_all(&resolved).await.map_err(|e| {
                        HostError::new(
                            "FS_IO",
                            format!("Failed to remove directory recursively: {}", e),
                        )
                        .into()
                    })
                } else {
                    fs::remove_dir(&resolved).await.map_err(|e| {
                        HostError::new("FS_IO", format!("Failed to remove directory: {}", e)).into()
                    })
                }
            } else {
                Err(HostError::new(rong::error::E_INTERNAL, "Unknown file type").into())
            }
        }
        Err(e) => Err(HostError::new("FS_IO", format!("Failed to access path: {}", e)).into()),
    }
}

async fn chdir(directory: String) -> JSResult<()> {
    let resolved = grant_file_access(&directory)?;
    std::env::set_current_dir(&resolved)
        .map_err(|e| HostError::new("FS_IO", format!("Failed to change directory: {}", e)).into())
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    let rong = ctx.host_namespace();

    ctx.register_hidden_class::<DirEntry>()?;

    let mkdir_fn = JSFunc::new(ctx, mkdir)?.name("mkdir")?;
    rong.set("mkdir", mkdir_fn)?;

    let remove_fn = JSFunc::new(ctx, remove)?.name("remove")?;
    rong.set("remove", remove_fn)?;

    let readdir_fn = JSFunc::new(ctx, readdir)?.name("readDir")?;
    rong.set("readDir", readdir_fn)?;

    let chdir_fn = JSFunc::new(ctx, chdir)?.name("chdir")?;
    rong.set("chdir", chdir_fn)?;

    Ok(())
}
