mod blob;
pub use blob::Blob;

use rusty_js::*;

pub fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<Blob>()?;
    Ok(())
}
