use rusty_js::*;

mod blob;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    blob::init(ctx)?;

    Ok(())
}
