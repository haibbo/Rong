use rong::{JSContext, JSResult};
use std::io::Write;

pub(crate) fn write_stderr_native(data: String) -> bool {
    eprint!("{data}");
    true
}

pub(crate) fn write_stdout_bytes_native(data: &[u8]) -> bool {
    std::io::stdout().write_all(data).is_ok()
}

pub(crate) fn write_stderr_bytes_native(data: &[u8]) -> bool {
    std::io::stderr().write_all(data).is_ok()
}

pub fn init(ctx: &JSContext) -> JSResult<()> {
    let _ = ctx;
    Ok(())
}
