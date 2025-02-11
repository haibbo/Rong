use rusty_js::*;

mod text_decoder;
mod text_encoder;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    text_encoder::init(ctx)?;
    text_decoder::init(ctx)?;

    Ok(())
}
