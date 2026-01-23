use std::sync::{Mutex, OnceLock};

/// Default User-Agent for Rong-hosted Web-ish APIs.
pub const DEFAULT_USER_AGENT: &str = concat!("RongJS/", env!("CARGO_PKG_VERSION"));

static USER_AGENT_SLOT: OnceLock<Mutex<String>> = OnceLock::new();

/// Set the process-global User-Agent string.
///
/// This validates it as an HTTP header value.
pub fn set_user_agent(ua: impl Into<String>) -> Result<(), String> {
    let ua_string = ua.into();
    http::HeaderValue::from_str(&ua_string)
        .map_err(|e| format!("invalid user agent header: {}", e))?;

    let slot = USER_AGENT_SLOT.get_or_init(|| Mutex::new(DEFAULT_USER_AGENT.to_string()));
    let mut guard = slot.lock().unwrap_or_else(|e| e.into_inner());
    *guard = ua_string;
    Ok(())
}

/// Get the current process-global User-Agent string.
pub fn get_user_agent() -> String {
    USER_AGENT_SLOT
        .get_or_init(|| Mutex::new(DEFAULT_USER_AGENT.to_string()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}
