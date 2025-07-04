use rong::*;

mod body;
mod fetch;
mod formdata;
mod header;
mod request;
mod response;
mod security;

// Re-export security-related items
pub use security::{NetworkAccessGuard, set_network_access_guard};

pub fn init(ctx: &JSContext) -> JSResult<()> {
    header::init(ctx)?;
    formdata::init(ctx)?;
    response::init(ctx)?;
    request::init(ctx)?;
    fetch::init(ctx)?;

    Ok(())
}
