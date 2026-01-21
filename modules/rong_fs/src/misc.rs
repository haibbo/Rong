use crate::grant_file_access;
use rong::*;
use std::time::SystemTime;
use tokio::fs;

/// Create a symbolic link
async fn symlink(old_path: String, new_path: String) -> JSResult<()> {
    let resolved_old = grant_file_access(&old_path)?;
    let resolved_new = grant_file_access(&new_path)?;
    #[cfg(unix)]
    {
        fs::symlink(&resolved_old, &resolved_new)
            .await
            .map_err(|e| HostError::new("FS_IO", format!("Failed to create symlink: {}", e)).into())
    }
    #[cfg(windows)]
    {
        // On Windows, we need to determine if the target is a directory
        match fs::metadata(&resolved_old).await {
            Ok(metadata) => {
                if metadata.is_dir() {
                    tokio::fs::symlink_dir(&resolved_old, &resolved_new)
                } else {
                    tokio::fs::symlink_file(&resolved_old, &resolved_new)
                }
            }
            Err(e) => Err(e),
        }
        .await
        .map_err(|e| HostError::new("FS_IO", format!("Failed to create symlink: {}", e)).into())
    }
}

/// Read the target of a symbolic link
async fn readlink(path: String) -> JSResult<String> {
    let resolved = grant_file_access(&path)?;
    fs::read_link(&resolved)
        .await
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| HostError::new("FS_IO", format!("Failed to read symlink: {}", e)).into())
}

/// Change file permissions (Unix only)
#[cfg(unix)]
async fn chmod(path: String, mode: u32) -> JSResult<()> {
    let resolved = grant_file_access(&path)?;
    use std::os::unix::fs::PermissionsExt;
    let permissions = std::fs::Permissions::from_mode(mode);
    fs::set_permissions(&resolved, permissions)
        .await
        .map_err(|e| HostError::new("FS_IO", format!("Failed to change permissions: {}", e)).into())
}

/// Change file ownership (Unix only)
#[cfg(unix)]
async fn chown(path: String, uid: u32, gid: u32) -> JSResult<()> {
    let resolved = grant_file_access(&path)?;
    use nix::unistd::{Gid, Uid, chown as nix_chown};
    nix_chown(
        &resolved,
        Some(Uid::from_raw(uid)),
        Some(Gid::from_raw(gid)),
    )
    .map_err(|e| HostError::new("FS_IO", format!("Failed to change ownership: {}", e)).into())
}

/// Options for utime function
#[derive(FromJSObj)]
pub(crate) struct UTimeOptions {
    accessed: Option<f64>,
    modified: Option<f64>,
}

/// Change file access and modification times
async fn utime(path: String, options: UTimeOptions) -> JSResult<()> {
    let resolved = grant_file_access(&path)?;
    use filetime::FileTime;

    let atime = options
        .accessed
        .map(|t| FileTime::from_unix_time((t / 1000.0) as i64, 0));
    let mtime = options
        .modified
        .map(|t| FileTime::from_unix_time((t / 1000.0) as i64, 0));

    filetime::set_file_times(
        &resolved,
        atime.unwrap_or_else(|| FileTime::from_system_time(SystemTime::now())),
        mtime.unwrap_or_else(|| FileTime::from_system_time(SystemTime::now())),
    )
    .map_err(|e| HostError::new("FS_IO", format!("Failed to set file times: {}", e)).into())
}

/// Rename a file or directory
async fn rename(from: String, to: String) -> JSResult<()> {
    let resolved_from = grant_file_access(&from)?;
    let resolved_to = grant_file_access(&to)?;
    fs::rename(&resolved_from, &resolved_to)
        .await
        .map_err(|e| HostError::new("FS_IO", format!("Failed to rename file: {}", e)).into())
}

/// Get the real path (canonical path) of a file
async fn real_path(path: String) -> JSResult<String> {
    let resolved = grant_file_access(&path)?;
    fs::canonicalize(&resolved)
        .await
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| HostError::new("FS_IO", format!("Failed to resolve real path: {}", e)).into())
}

/// Initialize miscellaneous file system functions
pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    let rong = ctx.rong();

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

    let utime_fn = JSFunc::new(ctx, utime)?.name("utime")?;
    rong.set("utime", utime_fn)?;

    let rename_fn = JSFunc::new(ctx, rename)?.name("rename")?;
    rong.set("rename", rename_fn)?;

    let real_path_fn = JSFunc::new(ctx, real_path)?.name("realPath")?;
    rong.set("realPath", real_path_fn)?;

    Ok(())
}
