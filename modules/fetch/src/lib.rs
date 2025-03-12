use rusty_js::*;

mod body;
mod fetch;
mod header;
mod request;
mod response;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    header::init(ctx)?;
    response::init(ctx)?;
    request::init(ctx)?;
    fetch::init(ctx)?;

    Ok(())
}
