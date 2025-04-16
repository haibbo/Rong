use rong_js::*;

pub use console;
pub use navigator;

/// Initialize all enabled modules in the JavaScript context
pub fn init(ctx: &JSContext) -> JSResult<()> {
    #[cfg(feature = "timer")]
    timer::init(ctx)?;

    #[cfg(feature = "navigator")]
    navigator::init(ctx)?;

    #[cfg(feature = "path")]
    path::init(ctx)?;

    #[cfg(feature = "fetch")]
    fetch::init(ctx)?;

    #[cfg(feature = "encoding")]
    encoding::init(ctx)?;

    #[cfg(feature = "event")]
    event::init(ctx)?;

    #[cfg(feature = "assert")]
    assert::init(ctx)?;

    #[cfg(feature = "dom-exception")]
    dom_exception::init(ctx)?;

    #[cfg(feature = "abort")]
    abort::init(ctx)?;

    #[cfg(feature = "console")]
    console::init(ctx)?;

    #[cfg(feature = "url")]
    rong_url::init(ctx)?;

    #[cfg(feature = "buffer")]
    buffer::init(ctx)?;

    #[cfg(feature = "fs")]
    fs::init(ctx)?;

    Ok(())
}
