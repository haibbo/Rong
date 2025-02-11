use rusty_js::*;

mod blob;
mod file;
mod header;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    blob::init(ctx)?;
    file::init(ctx)?;
    header::init(ctx)?;

    Ok(())
}
