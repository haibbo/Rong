use rong_js::*;

mod body;
mod fetch;
mod formdata;
mod header;
mod request;
mod response;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    header::init(ctx)?;
    formdata::init(ctx)?;
    response::init(ctx)?;
    request::init(ctx)?;
    fetch::init(ctx)?;

    Ok(())
}
