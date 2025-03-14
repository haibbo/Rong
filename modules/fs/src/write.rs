use rusty_js::{function::Optional, *};
use tokio::io::AsyncWriteExt;
use tokio::{fs, select};

use abort::AbortSignal;

#[derive(FromJSObj, Default)]
struct WriteFileOptions {
    // If set to true, will append to a file instead of overwriting previous contents
    append: Option<bool>,

    // If set to true, no file, directory, or symlink is allowed to exist at the
    // target location. When createNew is set to true, create is ignored.
    #[rename = "createNew"]
    create_new: Option<bool>,

    // Permissions always applied to file.
    mode: Option<u32>,

    // An abort signal to allow cancellation of the file write operation.
    signal: Option<AbortSignal>,
}

async fn write_text_file(
    file: String,
    text: String,
    option: Optional<WriteFileOptions>,
) -> JSResult<()> {
    let options = option.0.unwrap_or_default();

    // Handle createNew option
    if options.create_new.unwrap_or(false) && fs::metadata(&file).await.is_ok() {
        return Err(RustyJSError::TypeError("File already exists".into()));
    }

    // Handle append option
    let mut open_options = fs::OpenOptions::new();
    open_options
        .write(true)
        .create(true)
        .truncate(!options.append.unwrap_or(false))
        .append(options.append.unwrap_or(false));

    // Handle mode option (Unix-like systems, including macOS)
    #[cfg(unix)]
    if let Some(mode) = options.mode {
        open_options.mode(mode);
    }

    if let Some(abort) = options.signal {
        let mut abort = abort.subscribe();

        select! {
            result = async {
                let mut file = open_options.open(&file).await
                    .map_err(|e| RustyJSError::TypeError(format!("Failed to open file: {}", e)))?;
                file.write_all(text.as_bytes()).await
                    .map_err(|e| RustyJSError::TypeError(format!("Write failed: {}", e)))
            } => {
                result
            }

            abort_reason = abort.recv() => {
                Err(RustyJSError::from_jsvalue(abort_reason))
            }
        }
    } else {
        open_options
            .open(&file)
            .await
            .map_err(|e| RustyJSError::TypeError(format!("Failed to open file: {}", e)))?
            .write_all(text.as_bytes())
            .await
            .map_err(|e| RustyJSError::TypeError(format!("Write failed: {}", e)))
    }
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    let danity = ctx.dainty();

    let read = JSFunc::new(ctx, write_text_file)?.name("writeTextFile")?;
    danity.set("writeTextFile", read)?;
    Ok(())
}
