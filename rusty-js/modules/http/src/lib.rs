use rusty_js::*;

mod blob;
mod file;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    blob::init(ctx)?;
    file::init(ctx)?;

    Ok(())
}
