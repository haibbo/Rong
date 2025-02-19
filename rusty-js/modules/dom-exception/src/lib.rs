//! DOMException implementation
//!
//! This module provides a DOMException implementation that follows the Node.js error handling patterns.
//! It includes all standard error names used in Node.js.
//!
//! # Example
//! ```javascript
//! // Create a DOMException with a specific error type
//! const ex = new DOMException("Operation failed", "ABORT_ERR");
//! console.log(ex.name);    // "ABORT_ERR"
//! console.log(ex.message); // "Operation failed"
//! ```
//!
//! # Features
//! - Standard Node.js error names (INDEX_SIZE_ERR, ABORT_ERR, etc.)
//! - Full compatibility with Node.js error handling patterns
//!
//! # Error Categories
//! - DOM Hierarchy: INDEX_SIZE_ERR, HIERARCHY_REQUEST_ERR
//! - Data Handling: DOMSTRING_SIZE_ERR, DATA_CLONE_ERR
//! - State Management: INVALID_STATE_ERR, INVALID_ACCESS_ERR
//! - Network Operations: NETWORK_ERR, ABORT_ERR
//! - Resource Management: QUOTA_EXCEEDED_ERR, TIMEOUT_ERR
//!
//! //! # Notes
//!
//! The `code` property is **not implemented** in this module, as it has been
//! deprecated in the DOM specification and is no longer recommended for use.
//! Instead, use the `name` property to identify the type of error.

use rusty_js::{function::*, *};

/// Macro to define error names
#[allow(clippy::upper_case_acronyms)]
macro_rules! define_error_names {
    ($($name:ident),*) => {
        #[allow(clippy::upper_case_acronyms)]
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy)]
        pub enum DOMExceptionName {
            $($name,)*
        }

        impl DOMExceptionName {
            const ERROR_NAMES: &'static [&'static str] = &[$(stringify!($name)),*];

            /// Get the name as string
            #[inline]
            pub const fn as_str(&self) -> &'static str {
                Self::ERROR_NAMES[*self as usize]
            }

            /// Iterate over all error names efficiently
            #[inline]
            pub fn iter() -> impl Iterator<Item = &'static str> {
                Self::ERROR_NAMES.iter().copied()
            }
        }
    }
}

// This enum DOMExceptionName  represents all standard DOM exception names supported by Node.js
// Each variant corresponds to a specific error type in the DOM specification
define_error_names! {
    INDEX_SIZE_ERR,
    DOMSTRING_SIZE_ERR,
    HIERARCHY_REQUEST_ERR,
    INVALID_CHARACTER_ERR,
    NO_DATA_ALLOWED_ERR,
    NO_MODIFICATION_ALLOWED_ERR,
    NOT_FOUND_ERR,
    NOT_SUPPORTED_ERR,
    INUSE_ATTRIBUTE_ERR,
    INVALID_STATE_ERR,
    SYNTAX_ERR,
    INVALID_MODIFICATION_ERR,
    NAMESPACE_ERR,
    INVALID_ACCESS_ERR,
    VALIDATION_ERR,
    TYPE_MISMATCH_ERR,
    SECURITY_ERR,
    NETWORK_ERR,
    ABORT_ERR,
    URL_MISMATCH_ERR,
    QUOTA_EXCEEDED_ERR,
    TIMEOUT_ERR,
    INVALID_NODE_TYPE_ERR,
    DATA_CLONE_ERR
}

/// DOMException implementation following Node.js error types
#[js_class]
pub struct DOMException {
    name: String,
    message: String,
}

#[js_methods]
impl DOMException {
    #[js_method(constructor)]
    pub fn new(message: Optional<String>, name: Optional<String>) -> Self {
        let message = message.0.unwrap_or_default();
        let name = name.0.unwrap_or_else(|| "Error".to_string());

        Self { name, message }
    }

    #[js_method(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[js_method(getter)]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    #[js_method(getter)]
    pub fn stack(&self) -> String {
        "NotImplemented".to_string()
    }

    /// Create a new DOMException instance
    ///
    /// # Arguments
    /// * `message` - Error message
    /// * `name` - Error name
    ///
    /// # Returns
    /// Returns a JSObject containing the new DOMException instance
    pub fn create(ctx: JSContext, message: &str, name: &str) -> JSResult<JSObject> {
        let constructor = Class::get::<DOMException>(&ctx)?;
        let dom = DOMException::new(
            Optional(Some(message.to_string())),
            Optional(Some(name.to_string())),
        );
        Ok(constructor.instance::<DOMException>(dom))
    }
}

/// Register exception-related classes with the JavaScript engine
pub fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<DOMException>()?;
    let constructor = Class::get::<DOMException>(ctx)?;

    // Add all error names as static properties
    for name in DOMExceptionName::iter() {
        let desc = PropertyDescriptor::builder().value(JSValue::from(ctx, name));

        desc.apply_to(&constructor, name);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyjs_test::*;

    #[test]
    fn test_dom_exception() {
        async_run!(|ctx: JSContext| async move {
            init(&ctx)?;

            // Test constructor with name and message
            let result = ctx.eval::<JSObject>(Source::from_bytes(
                r#"
                const ex = new DOMException("Operation failed", "ABORT_ERR");
                ({
                    name: ex.name,
                    message: ex.message
                })
                "#,
            ))?;

            assert_eq!(result.get::<_, String>("name")?, "ABORT_ERR");
            assert_eq!(result.get::<_, String>("message")?, "Operation failed");

            Ok(())
        });
    }
}
