use rusty_js::*;

mod blob;
mod body;
mod file;
mod header;
mod request;
mod response;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    blob::init(ctx)?;
    file::init(ctx)?;
    header::init(ctx)?;
    request::init(ctx)?;

    Ok(())
}
