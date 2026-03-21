use rong::*;

#[cfg(feature = "worker")]
pub use rong_worker as worker;

/// Initialize all enabled modules in the JavaScript context
pub fn init(ctx: &JSContext) -> JSResult<()> {
    #[cfg(feature = "timer")]
    rong_timer::init(ctx)?;

    #[cfg(feature = "stream")]
    rong_stream::init(ctx)?;

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

    #[cfg(feature = "process")]
    rong_process::init(ctx)?;

    #[cfg(feature = "child_process")]
    rong_child_process::init(ctx)?;

    #[cfg(feature = "redis")]
    rong_redis::init(ctx)?;

    #[cfg(feature = "sqlite")]
    rong_sqlite::init(ctx)?;

    #[cfg(feature = "worker")]
    rong_worker::init(ctx)?;

    Ok(())
}
