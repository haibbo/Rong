use rusty_js::{function::*, *};
use std::path::{Component, Path, PathBuf};

pub fn init(ctx: &JSContext) -> JSResult<()> {
    let path = JSObject::new(ctx);

    // Basic path operations
    path.set("basename", ctx.register_function(basename));
    path.set("dirname", ctx.register_function(dirname));
    path.set("extname", ctx.register_function(extname));
    path.set("isAbsolute", ctx.register_function(is_absolute));

    // Path combination
    path.set("join", ctx.register_function(join));
    path.set("resolve", ctx.register_function(resolve));
    path.set("normalize", ctx.register_function(normalize));

    // Path parsing
    path.set("parse", ctx.register_function(parse));
    path.set("format", ctx.register_function(format));

    // Platform-specific
    path.set("sep", std::path::MAIN_SEPARATOR.to_string());
    path.set("delimiter", if cfg!(windows) { ";" } else { ":" });

    ctx.global().set("path", path);
    Ok(())
}

/// Get the last portion of a path
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

/// Get the directory name of a path
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

/// Get the extension of a path
fn extname(path: String) -> String {
    let path = Path::new(&path);
    path.extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default()
}

/// Check if a path is absolute
fn is_absolute(path: String) -> bool {
    Path::new(&path).is_absolute()
}

/// Normalize path components
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

/// Join path segments
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

/// Resolve a path
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

/// Normalize a path
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

/// Parse a path into an object
fn parse(ctx: &JSContext, path: String) -> JSResult<JSObject> {
    let path = Path::new(&path);
    let obj = JSObject::new(ctx);

    // Handle root
    obj.set(
        "root",
        if path.is_absolute() {
            "/".to_string()
        } else {
            String::new()
        },
    );

    obj.set("dir", dirname(path.to_string_lossy().into_owned()));
    obj.set(
        "base",
        basename(path.to_string_lossy().into_owned(), Optional(None)),
    );
    obj.set("ext", extname(path.to_string_lossy().into_owned()));
    obj.set(
        "name",
        path.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );

    Ok(obj)
}

/// Format a path from an object
fn format(path_objecg: JSObject) -> JSResult<String> {
    let mut path_buf = PathBuf::new();

    // Process parts in priority order
    if let Ok(root) = path_objecg.get::<_, String>("root") {
        if !root.is_empty() {
            path_buf.push(root);
        }
    }

    if let Ok(dir) = path_objecg.get::<_, String>("dir") {
        if !dir.is_empty() {
            path_buf.push(dir);
        }
    }

    if let Ok(base) = path_objecg.get::<_, String>("base") {
        if !base.is_empty() {
            path_buf.push(base);
        }
    } else {
        if let Ok(name) = path_objecg.get::<_, String>("name") {
            if !name.is_empty() {
                path_buf.push(name);
            }
        }
        if let Ok(ext) = path_objecg.get::<_, String>("ext") {
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
    use rustyjs_test::*;

    #[test]
    fn test_path() {
        async_run!(|ctx: JSContext| async move {
            ctx.global().set(
                "print",
                JSFunc::new(&ctx, |msg: String| println!("JS: {}", msg)),
            );

            init(&ctx).unwrap();

            let source = Source::from_path("tests/path.js").await.unwrap();
            let obj: JSObject = ctx.eval_async(source).await?;

            let total: i32 = obj.get("total").unwrap();
            let passed: i32 = obj.get("passed").unwrap();
            let success: bool = obj.get("success").unwrap();

            if !success {
                let failed: JSArray = obj.get("failed").unwrap();
                let error_messages: Vec<String> = failed.iter().collect::<JSResult<_>>()?;
                panic!(
                    "Path tests failed:\nPassed {}/{}\nFailures:\n{}",
                    passed,
                    total,
                    error_messages.join("\n")
                );
            }
            Ok(())
        });
    }
}
