use rusty_js::{function::Optional, *};
use tokio::{fs, select};

use abort::AbortSignal;

#[derive(FromJSObj)]
struct ReadFileOptions {
    signal: AbortSignal,
}

async fn read_text_file(file: String, option: Optional<ReadFileOptions>) -> JSResult<String> {
    if let Some(abort) = option.0 {
        let mut abort = abort.signal.subscribe();

        select! {
            result = fs::read_to_string(file) => {
                result.into_result()
            }

            abort_reason = abort.recv() => {
                Err(RustyJSError::from_jsvalue(abort_reason))
            }
        }
    } else {
        fs::read_to_string(file).await.into_result()
    }
}

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    let danity = ctx.dainty();

    let read = JSFunc::new(ctx, read_text_file)?.name("readTextFile")?;
    danity.set("readTextFile", read)?;
    Ok(())
}
