use std::fs;
use std::path::Path;

/// Represents a source of JavaScript code that can be evaluated
#[derive(Debug, Clone)]
pub struct Source {
    /// The actual JavaScript code in UTF-8 bytes
    code: Vec<u8>,
    /// Optional name/path for error reporting
    name: Option<String>,
}

impl Source {
    /// Create a Source from UTF-8 bytes
    pub fn from_bytes(code: impl Into<Vec<u8>>) -> Self {
        Self {
            code: code.into(),
            name: None,
        }
    }

    /// Create a Source from a file path
    pub fn from_path(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let code = fs::read(path.as_ref())?;
        Ok(Self {
            code,
            name: Some(path.as_ref().to_string_lossy().into_owned()),
        })
    }

    /// Get the source code as UTF-8 bytes
    pub fn code(&self) -> &[u8] {
        &self.code
    }

    /// Get the source name/path if available
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}
