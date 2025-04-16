use futures::Stream;
use rong::{function::Optional, *};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::SystemTime;
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
    fn new(name: String, file_type: bool, is_symlink: bool) -> Self {
        Self {
            name,
            file_type,
            is_symlink,
        }
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
}

#[derive(FromJSObj, Default)]
struct MkdirOptions {
    // If true, parent folders will be created if they don't exist
    recursive: Option<bool>,
    // Permissions to set on the created directory
    mode: Option<u32>,
}

async fn mkdir(path: String, option: Optional<MkdirOptions>) -> JSResult<()> {
    let options = option.0.unwrap_or_default();

    // Check if directory exists first
    if let Ok(metadata) = fs::metadata(&path).await {
        if metadata.is_dir() {
            return Ok(()); // Directory already exists, return success
        }
    }

    let result = if options.recursive.unwrap_or(false) {
        fs::create_dir_all(&path).await
    } else {
        fs::create_dir(&path).await
    };

    result.map_err(|e| RongJSError::TypeError(format!("Failed to create directory: {}", e)))?;

    // Set mode if specified (Unix-like systems only)
    #[cfg(unix)]
    if let Some(mode) = options.mode {
        use std::os::unix::fs::PermissionsExt;
        let permissions = std::fs::Permissions::from_mode(mode);
        tokio::fs::set_permissions(&path, permissions)
            .await
            .map_err(|e| {
                RongJSError::TypeError(format!("Failed to set directory permissions: {}", e))
            })?;
    }

    Ok(())
}

pub struct DirEntryStream {
    entries: fs::ReadDir,
    current_entry: Option<fs::DirEntry>,
    current_file_type_fut: Option<
        Pin<Box<dyn futures::Future<Output = Result<std::fs::FileType, std::io::Error>> + Send>>,
    >,
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
                    return Poll::Ready(Some(Err(RongJSError::TypeError(format!(
                        "Failed to get file type: {}",
                        e
                    )))));
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
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(RongJSError::TypeError(format!(
                "Failed to read directory entry: {}",
                e
            ))))),
            Poll::Pending => Poll::Pending,
        }
    }
}

async fn readdir(ctx: JSContext, path: String) -> JSResult<JSObject> {
    let entries = fs::read_dir(&path)
        .await
        .map_err(|e| RongJSError::TypeError(format!("Failed to read directory: {}", e)))?;

    let stream = DirEntryStream::new(entries);
    stream.into_js_async_iter(&ctx)
}

#[derive(FromJSObj, Default)]
struct RemoveOptions {
    // If set to true, path will be removed even if it's a non-empty directory.
    recursive: bool,
}

async fn remove(path: String, option: Optional<RemoveOptions>) -> JSResult<()> {
    let options = option.0.unwrap_or_default();

    // Check if path exists and get its type
    match fs::metadata(&path).await {
        Ok(metadata) => {
            if metadata.is_file() || metadata.is_symlink() {
                fs::remove_file(&path)
                    .await
                    .map_err(|e| RongJSError::TypeError(format!("Failed to remove file: {}", e)))
            } else if metadata.is_dir() {
                if options.recursive {
                    fs::remove_dir_all(&path).await.map_err(|e| {
                        RongJSError::TypeError(format!(
                            "Failed to remove directory recursively: {}",
                            e
                        ))
                    })
                } else {
                    fs::remove_dir(&path).await.map_err(|e| {
                        RongJSError::TypeError(format!("Failed to remove directory: {}", e))
                    })
                }
            } else {
                Err(RongJSError::TypeError("Unknown file type".to_string()))
            }
        }
        Err(e) => Err(RongJSError::TypeError(format!(
            "Failed to access path: {}",
            e
        ))),
    }
}

async fn symlink(old_path: String, new_path: String) -> JSResult<()> {
    #[cfg(unix)]
    {
        fs::symlink(&old_path, &new_path)
            .await
            .map_err(|e| RongJSError::TypeError(format!("Failed to create symlink: {}", e)))
    }
    #[cfg(windows)]
    {
        // On Windows, we need to determine if the target is a directory
        match fs::metadata(&old_path).await {
            Ok(metadata) => {
                if metadata.is_dir() {
                    tokio::fs::symlink_dir(&old_path, &new_path)
                } else {
                    tokio::fs::symlink_file(&old_path, &new_path)
                }
            }
            Err(e) => Err(e),
        }
        .await
        .map_err(|e| RongJSError::TypeError(format!("Failed to create symlink: {}", e)))
    }
}

async fn readlink(path: String) -> JSResult<String> {
    fs::read_link(&path)
        .await
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| RongJSError::TypeError(format!("Failed to read symlink: {}", e)))
}

#[cfg(unix)]
async fn chmod(path: String, mode: u32) -> JSResult<()> {
    use std::os::unix::fs::PermissionsExt;
    let permissions = std::fs::Permissions::from_mode(mode);
    fs::set_permissions(&path, permissions)
        .await
        .map_err(|e| RongJSError::TypeError(format!("Failed to change permissions: {}", e)))
}

#[cfg(unix)]
async fn chown(path: String, uid: u32, gid: u32) -> JSResult<()> {
    use nix::unistd::{chown as nix_chown, Gid, Uid};
    nix_chown(
        path.as_str(),
        Some(Uid::from_raw(uid)),
        Some(Gid::from_raw(gid)),
    )
    .map_err(|e| RongJSError::TypeError(format!("Failed to change ownership: {}", e)))
}

async fn chdir(directory: String) -> JSResult<()> {
    std::env::set_current_dir(&directory)
        .map_err(|e| RongJSError::TypeError(format!("Failed to change directory: {}", e)))
}

#[derive(FromJSObj)]
struct UTimeOptions {
    accessed: Option<f64>,
    modified: Option<f64>,
}

async fn utime(path: String, options: UTimeOptions) -> JSResult<()> {
    use filetime::FileTime;

    let atime = options
        .accessed
        .map(|t| FileTime::from_unix_time((t / 1000.0) as i64, 0));
    let mtime = options
        .modified
        .map(|t| FileTime::from_unix_time((t / 1000.0) as i64, 0));

    filetime::set_file_times(
        &path,
        atime.unwrap_or_else(|| FileTime::from_system_time(SystemTime::now())),
        mtime.unwrap_or_else(|| FileTime::from_system_time(SystemTime::now())),
    )
    .map_err(|e| RongJSError::TypeError(format!("Failed to set file times: {}", e)))
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    let rong = ctx.rong();

    ctx.register_class::<DirEntry>()?;

    let mkdir_fn = JSFunc::new(ctx, mkdir)?.name("mkdir")?;
    rong.set("mkdir", mkdir_fn)?;

    let remove_fn = JSFunc::new(ctx, remove)?.name("remove")?;
    rong.set("remove", remove_fn)?;

    let readdir_fn = JSFunc::new(ctx, readdir)?.name("readDir")?;
    rong.set("readDir", readdir_fn)?;

    let symlink_fn = JSFunc::new(ctx, symlink)?.name("symlink")?;
    rong.set("symlink", symlink_fn)?;

    let readlink_fn = JSFunc::new(ctx, readlink)?.name("readlink")?;
    rong.set("readlink", readlink_fn)?;

    #[cfg(unix)]
    {
        let chmod_fn = JSFunc::new(ctx, chmod)?.name("chmod")?;
        rong.set("chmod", chmod_fn)?;

        let chown_fn = JSFunc::new(ctx, chown)?.name("chown")?;
        rong.set("chown", chown_fn)?;
    }

    let chdir_fn = JSFunc::new(ctx, chdir)?.name("chdir")?;
    rong.set("chdir", chdir_fn)?;

    let utime_fn = JSFunc::new(ctx, utime)?.name("utime")?;
    rong.set("utime", utime_fn)?;

    Ok(())
}
