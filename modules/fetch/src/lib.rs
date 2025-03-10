use rusty_js::*;

mod blob;
mod body;
mod fetch;
mod file;
mod header;
mod request;
mod response;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    blob::init(ctx)?;
    file::init(ctx)?;
    header::init(ctx)?;
    response::init(ctx)?;
    request::init(ctx)?;
    fetch::init(ctx)?;

    Ok(())
}
