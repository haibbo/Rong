use rong::*;

mod body;
mod client;
mod download;
mod fetch;
mod formdata;
mod header;
mod request;
mod response;
mod security;

pub use client::*;
pub use download::*;

// Re-export security-related items
pub use security::{
    NetworkAccessGuard, set_network_access_guard, set_network_access_guard_scoped,
};

pub fn init(ctx: &JSContext) -> JSResult<()> {
    header::init(ctx)?;
    formdata::init(ctx)?;
    response::init(ctx)?;
    request::init(ctx)?;
    fetch::init(ctx)?;

    Ok(())
}
