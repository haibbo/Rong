//! Path module for RongJS
//!
//! This module provides functionality for working with file and directory paths.
//! It aims to be compatible with Node.js's path module API, providing similar
//! functionality for path manipulation and normalization.
//!
//! # Features
//!
//! - Basic path operations (basename, dirname, extname)
//! - Path combination and normalization (join, resolve, normalize)
//! - Path parsing and formatting
//! - Platform-specific path handling
//!
//! # Example
//!
//! ```javascript
//! // Get the last portion of a path
//! path.basename('/foo/bar/baz.html');     // Returns: 'baz.html'
//! path.basename('/foo/bar/baz.html', '.html');  // Returns: 'baz'
//!
//! // Get directory name
//! path.dirname('/foo/bar/baz');  // Returns: '/foo/bar'
//!
//! // Join path segments
//! path.join('/foo', 'bar', 'baz');  // Returns: '/foo/bar/baz'
//! ```

use rong::{function::*, *};
use std::path::{Component, Path, PathBuf};

pub fn init(ctx: &JSContext) -> JSResult<()> {
    let path = JSObject::new(ctx);

    // Basic path operations
    path.set("basename", JSFunc::new(ctx, basename))?;
    path.set("dirname", JSFunc::new(ctx, dirname))?;
    path.set("extname", JSFunc::new(ctx, extname))?;
    path.set("isAbsolute", JSFunc::new(ctx, is_absolute))?;

    // Path combination
    path.set("join", JSFunc::new(ctx, join))?;
    path.set("resolve", JSFunc::new(ctx, resolve))?;
    path.set("normalize", JSFunc::new(ctx, normalize))?;

    // Path parsing
    path.set("parse", JSFunc::new(ctx, parse))?;
    path.set("format", JSFunc::new(ctx, format))?;

    // Platform-specific
    path.set("sep", std::path::MAIN_SEPARATOR.to_string())?;
    path.set("delimiter", if cfg!(windows) { ";" } else { ":" })?;

    ctx.global().set("path", path)?;
    Ok(())
}

/// Returns the last portion of a path.
///
/// # Arguments
///
/// * `path` - The path to process
/// * `suffix` - Optional suffix to remove from the result
///
/// # Examples
///
/// ```javascript
/// path.basename('/foo/bar/baz.html')         // Returns: 'baz.html'
/// path.basename('/foo/bar/baz.html', '.html') // Returns: 'baz'
/// path.basename('/foo/bar/baz')              // Returns: 'baz'
/// path.basename('/foo/bar/')                 // Returns: ''
/// ```
fn basename(path: String, suffix: Optional<String>) -> String {
    let path = Path::new(&path);
    let file_name = path.file_name().map(|s| s.to_string_lossy().into_owned());

    if let Some(name) = file_name {
        if let Some(suffix) = suffix.0 {
            if name.ends_with(&suffix) {
                return name[..name.len() - suffix.len()].to_string();
            }
        }
        name
    } else {
        String::new()
    }
}

/// Returns the directory name of a path.
///
/// # Arguments
///
/// * `path` - The path to process
///
/// # Examples
///
/// ```javascript
/// path.dirname('/foo/bar/baz')     // Returns: '/foo/bar'
/// path.dirname('/foo/bar/baz/')    // Returns: '/foo/bar'
/// path.dirname('/foo')             // Returns: '/'
/// path.dirname('foo')              // Returns: '.'
/// path.dirname('')                 // Returns: '.'
/// ```
fn dirname(path: String) -> String {
    if path.is_empty() {
        return ".".to_string();
    }
    let path = Path::new(&path);
    path.parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                if path.is_absolute() {
                    String::from("/")
                } else {
                    String::from(".")
                }
            } else {
                p.to_string_lossy().into_owned()
            }
        })
        .unwrap_or_else(|| String::from("."))
}

/// Returns the extension of a path.
///
/// # Arguments
///
/// * `path` - The path to process
///
/// # Examples
///
/// ```javascript
/// path.extname('index.html')      // Returns: '.html'
/// path.extname('index.coffee.md') // Returns: '.md'
/// path.extname('index.')          // Returns: '.'
/// path.extname('index')           // Returns: ''
/// path.extname('.index')          // Returns: ''
/// ```
fn extname(path: String) -> String {
    let path = Path::new(&path);
    path.extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default()
}

/// Determines if a path is absolute.
///
/// # Arguments
///
/// * `path` - The path to check
///
/// # Examples
///
/// ```javascript
/// path.isAbsolute('/foo/bar')    // Returns: true
/// path.isAbsolute('foo/bar')     // Returns: false
/// path.isAbsolute('./foo/bar')   // Returns: false
/// ```
fn is_absolute(path: String) -> bool {
    Path::new(&path).is_absolute()
}

/// Normalizes path components by resolving '..' and '.' segments.
///
/// # Arguments
///
/// * `path` - The path to normalize
///
/// # Returns
///
/// * `PathBuf` - A normalized path
fn normalize_components(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                if !components.is_empty()
                    && !matches!(components.last(), Some(Component::ParentDir))
                    && !matches!(components.last(), Some(Component::RootDir))
                {
                    components.pop();
                } else {
                    components.push(component);
                }
            }
            Component::Normal(_) | Component::RootDir | Component::Prefix(_) => {
                components.push(component);
            }
            Component::CurDir => {}
        }
    }
    components.iter().collect()
}

/// Joins all given path segments together.
///
/// # Arguments
///
/// * `args` - A list of path segments to join
///
/// # Examples
///
/// ```javascript
/// path.join('/foo', 'bar', 'baz')    // Returns: '/foo/bar/baz'
/// path.join('/foo', 'bar', '../baz')  // Returns: '/foo/baz'
/// path.join('foo', 'bar', 'baz')      // Returns: 'foo/bar/baz'
/// ```
fn join(args: Rest<String>) -> JSResult<String> {
    if args.0.is_empty() {
        return Ok(".".to_string());
    }

    let mut path_buf = PathBuf::new();
    for arg in args.0 {
        if !arg.is_empty() {
            path_buf.push(arg);
        }
    }

    if path_buf.as_os_str().is_empty() {
        Ok(".".to_string())
    } else {
        let normalized = normalize_components(&path_buf);
        Ok(normalized.to_string_lossy().into_owned())
    }
}

/// Resolves a sequence of paths to an absolute path.
///
/// # Arguments
///
/// * `args` - A list of paths to resolve
///
/// # Examples
///
/// ```javascript
/// // On UNIX:
/// path.resolve('/foo/bar', './baz')   // Returns: '/foo/bar/baz'
/// path.resolve('/foo/bar', '/baz')    // Returns: '/baz'
/// ```
fn resolve(args: Rest<String>) -> JSResult<String> {
    let mut path_buf = if cfg!(windows) {
        PathBuf::from("C:\\") // Default starting point for Windows
    } else {
        PathBuf::from("/") // Default starting point for Unix
    };

    for segment_path in args.0 {
        let path = PathBuf::from(&segment_path);
        if path.is_absolute() {
            path_buf = path;
        } else {
            path_buf.push(segment_path);
        }
    }

    Ok(path_buf.to_string_lossy().into_owned())
}

/// Normalizes a path by resolving '..' and '.' segments.
///
/// # Arguments
///
/// * `path` - The path to normalize
///
/// # Examples
///
/// ```javascript
/// path.normalize('/foo/bar//baz/asdf/quux/..')  // Returns: '/foo/bar/baz/asdf'
/// path.normalize('foo/bar//baz/asdf/quux/..')   // Returns: 'foo/bar/baz/asdf'
/// ```
fn normalize(path: String) -> String {
    if path.is_empty() {
        return ".".to_string();
    }

    let path = Path::new(&path);
    let normalized = normalize_components(path);
    if normalized.as_os_str().is_empty() {
        ".".to_string()
    } else {
        normalized.to_string_lossy().into_owned()
    }
}

/// Parses a path into an object with root, dir, base, ext, and name properties.
///
/// # Arguments
///
/// * `ctx` - The JavaScript context
/// * `path` - The path to parse
///
/// # Examples
///
/// ```javascript
/// path.parse('/home/user/dir/file.txt')
/// // Returns:
/// // {
/// //    root: '/',
/// //    dir: '/home/user/dir',
/// //    base: 'file.txt',
/// //    ext: '.txt',
/// //    name: 'file'
/// // }
/// ```
fn parse(ctx: JSContext, path: String) -> JSResult<JSObject> {
    let path = Path::new(&path);
    let obj = JSObject::new(&ctx);

    // Handle root
    obj.set(
        "root",
        if path.is_absolute() {
            "/".to_string()
        } else {
            String::new()
        },
    )?;

    obj.set("dir", dirname(path.to_string_lossy().into_owned()))?;
    obj.set(
        "base",
        basename(path.to_string_lossy().into_owned(), Optional(None)),
    )?;
    obj.set("ext", extname(path.to_string_lossy().into_owned()))?;
    obj.set(
        "name",
        path.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
    )?;

    Ok(obj)
}

/// Formats a path object into a path string.
///
/// # Arguments
///
/// * `path_object` - An object with path properties (root, dir, base, name, ext)
///
/// # Examples
///
/// ```javascript
/// path.format({
///     root: '/',
///     dir: '/home/user/dir',
///     base: 'file.txt'
/// })
/// // Returns: '/home/user/dir/file.txt'
/// ```
fn format(path_object: JSObject) -> JSResult<String> {
    let mut path_buf = PathBuf::new();

    // Process parts in priority order
    if let Ok(root) = path_object.get::<_, String>("root") {
        if !root.is_empty() {
            path_buf.push(root);
        }
    }

    if let Ok(dir) = path_object.get::<_, String>("dir") {
        if !dir.is_empty() {
            path_buf.push(dir);
        }
    }

    if let Ok(base) = path_object.get::<_, String>("base") {
        if !base.is_empty() {
            path_buf.push(base);
        }
    } else {
        if let Ok(name) = path_object.get::<_, String>("name") {
            if !name.is_empty() {
                path_buf.push(name);
            }
        }
        if let Ok(ext) = path_object.get::<_, String>("ext") {
            if !ext.is_empty() {
                let file_name = path_buf
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default()
                    + &ext;
                path_buf.set_file_name(file_name);
            }
        }
    }

    Ok(path_buf.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong_test::*;

    #[test]
    fn test_path() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx)?;
            rong_console::init(&ctx)?;

            let passed = UnitJSRunner::load_script(&ctx, "path.js")
                .await?
                .run()
                .await?;
            assert!(passed);
            Ok(())
        });
    }
}
