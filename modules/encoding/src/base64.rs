use base64::{engine::general_purpose, Engine as _};
use rusty_js::{JSResult, RustyJSError};

/// Decodes a string of data which has been encoded using base-64 encoding
pub fn atob(input: String) -> JSResult<String> {
    let decoded = general_purpose::STANDARD
        .decode(input)
        .map_err(|e| RustyJSError::TypeError(format!("Failed to decode base64: {}", e)))?;
    let decoded_str = String::from_utf8(decoded)
        .map_err(|e| RustyJSError::TypeError(format!("Invalid UTF-8 sequence: {}", e)))?;
    Ok(decoded_str)
}

/// Creates a base-64 ASCII encoded string from the input string
pub fn btoa(input: String) -> JSResult<String> {
    let encoded = general_purpose::STANDARD.encode(input);
    Ok(encoded)
}
