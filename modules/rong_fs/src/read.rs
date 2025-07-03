use rong::{function::Optional, *};
use tokio::{fs, select};

use crate::grant_file_access;
use rong_abort::AbortSignal;

#[derive(FromJSObj)]
struct ReadFileOptions {
    signal: Option<AbortSignal>,
}

async fn read_text_file(file: String, option: Optional<ReadFileOptions>) -> JSResult<String> {
    grant_file_access(&file)?;
    let options = option.0.unwrap_or(ReadFileOptions { signal: None });

    if let Some(signal) = options.signal {
        let mut abort = signal.subscribe();

        select! {
            result = fs::read_to_string(&file) => {
                result.into_result()
            }

            abort_reason = abort.recv() => {
                // println!("read_text_file: Received abort signal");
                Err(RongJSError::from_jsvalue(abort_reason))
            }
        }
    } else {
        fs::read_to_string(file).await.into_result()
    }
}

async fn read_file(
    ctx: JSContext,
    file: String,
    option: Optional<ReadFileOptions>,
) -> JSResult<JSArrayBuffer<u8>> {
    grant_file_access(&file)?;
    let options = option.0.unwrap_or(ReadFileOptions { signal: None });

    if let Some(signal) = options.signal {
        let mut abort = signal.subscribe();
        println!("read_file: Subscribed to abort signal");

        select! {
            result = fs::read(&file) => {
                println!("read_file: File read completed");
                match result {
                    Ok(bytes) => JSArrayBuffer::<u8>::from_bytes_owned(&ctx, bytes),
                    Err(e) => Err(RongJSError::TypeError(format!("Failed to read file: {}", e)))
                }
            }

            abort_reason = abort.recv() => {
                println!("read_file: Received abort signal");
                Err(RongJSError::from_jsvalue(abort_reason))
            }
        }
    } else {
        let bytes = fs::read(file)
            .await
            .map_err(|e| RongJSError::TypeError(format!("Failed to read file: {}", e)))?;

        JSArrayBuffer::<u8>::from_bytes_owned(&ctx, bytes)
    }
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    let rong = ctx.rong();

    let read_text = JSFunc::new(ctx, read_text_file)?.name("readTextFile")?;
    rong.set("readTextFile", read_text)?;

    let read_file = JSFunc::new(ctx, read_file)?.name("readFile")?;
    rong.set("readFile", read_file)?;

    Ok(())
}
