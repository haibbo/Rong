use rusty_js::*;

mod blob;
mod file;
mod header;
mod request;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    blob::init(ctx)?;
    file::init(ctx)?;
    header::init(ctx)?;
    request::init(ctx)?;

    Ok(())
}
