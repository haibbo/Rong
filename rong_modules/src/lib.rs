use rong::*;

#[cfg(feature = "abort")]
pub use rong_abort as abort;
#[cfg(feature = "assert")]
pub use rong_assert as assert;
#[cfg(feature = "buffer")]
pub use rong_buffer as buffer;
#[cfg(feature = "console")]
pub use rong_console as console;
#[cfg(feature = "encoding")]
pub use rong_encoding as encoding;
#[cfg(feature = "event")]
pub use rong_event as event;
#[cfg(feature = "exception")]
pub use rong_exception as exception;
#[cfg(feature = "fs")]
pub use rong_fs as fs;
#[cfg(feature = "http")]
pub use rong_http as http;
#[cfg(feature = "navigator")]
pub use rong_navigator as navigator;
#[cfg(feature = "path")]
pub use rong_path as path;
#[cfg(feature = "storage")]
pub use rong_storage as storage;
#[cfg(feature = "timer")]
pub use rong_timer as timer;
#[cfg(feature = "url")]
pub use rong_url as url;

/// Initialize all enabled modules in the JavaScript context
pub fn init(ctx: &JSContext) -> JSResult<()> {
    #[cfg(feature = "timer")]
    rong_timer::init(ctx)?;

    #[cfg(feature = "navigator")]
    rong_navigator::init(ctx)?;

    #[cfg(feature = "path")]
    rong_path::init(ctx)?;

    #[cfg(feature = "http")]
    rong_http::init(ctx)?;

    #[cfg(feature = "encoding")]
    rong_encoding::init(ctx)?;

    #[cfg(feature = "event")]
    rong_event::init(ctx)?;

    #[cfg(feature = "assert")]
    rong_assert::init(ctx)?;

    #[cfg(feature = "exception")]
    rong_exception::init(ctx)?;

    #[cfg(feature = "abort")]
    rong_abort::init(ctx)?;

    #[cfg(feature = "console")]
    rong_console::init(ctx)?;

    #[cfg(feature = "url")]
    rong_url::init(ctx)?;

    #[cfg(feature = "buffer")]
    rong_buffer::init(ctx)?;

    #[cfg(feature = "fs")]
    rong_fs::init(ctx)?;

    #[cfg(feature = "storage")]
    rong_storage::init(ctx)?;
    Ok(())
}
