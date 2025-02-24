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

            /// Get the name as string with automatic conversion
            #[inline]
            pub fn as_str(&self) -> &'static str {
                let name = Self::ERROR_NAMES[*self as usize];
                // Convert "ERR" suffix to "Error" and remove underscores
                if let Some(base) = name.strip_suffix("_ERR") {
                    let mut result = String::with_capacity(base.len() + 5);
                    for part in base.split('_') {
                        if !part.is_empty() {
                            result.push_str(&part[0..1].to_uppercase());
                            result.push_str(&part[1..].to_lowercase());
                        }
                    }
                    result.push_str("Error");
                    return Box::leak(result.into_boxed_str());
                }
                name
            }

            /// Iterate over all error names efficiently
            #[inline]
            pub fn iter() -> impl Iterator<Item = &'static str> {
                Self::ERROR_NAMES.iter().map(|&name| {
                    Self::find_or_default(name).as_str()
                })
            }

            /// Convert string to corresponding DOMExceptionName variant
            /// Default to ERROR variant if no match found
            #[inline]
            pub fn find_or_default(s: &str) -> Self {
                match s {
                $(stringify!($name) => DOMExceptionName::$name,)*
                _ => DOMExceptionName::ERROR,
                }
            }
        }
    }
}

// This enum DOMExceptionName represents all standard DOM exception names supported by Node.js
// https://webidl.spec.whatwg.org/#idl-DOMException-error-names
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
    DATA_CLONE_ERR,
    ERROR
}

// Implement From<&str> to replace from_str
impl From<&str> for DOMExceptionName {
    fn from(s: &str) -> Self {
        DOMExceptionName::find_or_default(s)
    }
}

// Implement From<Option<String>> using From<&str>
impl From<Option<String>> for DOMExceptionName {
    fn from(value: Option<String>) -> Self {
        match value {
            Some(s) => s.as_str().into(),
            None => DOMExceptionName::ERROR,
        }
    }
}

/// DOMException implementation following Node.js error types
#[js_class]
pub struct DOMException {
    name: DOMExceptionName,
    message: String,
}

#[js_methods]
impl DOMException {
    #[js_method(constructor)]
    pub fn new(message: Optional<String>, name: Optional<String>) -> Self {
        Self {
            message: message.0.unwrap_or_default(),
            name: name.0.into(),
        }
    }

    #[js_method(getter)]
    pub fn name(&self) -> String {
        self.name.as_str().to_string()
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
    /// * `name` - DOMExceptionName
    ///
    /// # Returns
    /// Returns a JSObject containing the new DOMException instance
    pub fn create(ctx: &JSContext, message: &str, name: DOMExceptionName) -> JSResult<JSObject> {
        let dom = DOMException {
            message: message.to_string(),
            name,
        };
        Ok(Class::get::<DOMException>(ctx)?.instance(dom))
    }
}

/// Register exception-related classes with the JavaScript engine
pub fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<DOMException>()?;
    let constructor = Class::get::<DOMException>(ctx)?;

    // Add all error names as static properties
    for name in DOMExceptionName::iter() {
        PropertyDescriptor::builder()
            .value(JSValue::from(ctx, name))
            .enumerable(true)
            .writable(false)
            .configurable(false)
            .apply_to(&constructor, name);
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

            assert_eq!(result.get::<_, String>("name")?, "AbortError");
            assert_eq!(result.get::<_, String>("message")?, "Operation failed");

            Ok(())
        });
    }
}
