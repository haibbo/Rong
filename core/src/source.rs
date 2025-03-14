use std::path::Path;

#[derive(Debug, Clone)]
pub enum SourceKind {
    JavaScript(Vec<u8>), // UTF-8 JavaScript code
    ByteCode(Vec<u8>),   // Compiled bytecode
}

#[derive(Debug, Clone)]
pub struct Source {
    kind: SourceKind,
    name: Option<String>,
}

impl Source {
    /// Create a Source from JavaScript code
    ///
    /// # Arguments
    /// * `code` - The JavaScript source code. Accepts:
    ///   - &str: JavaScript source code as string
    ///   - &[u8]: JavaScript source code as bytes
    ///   - String: Owned JavaScript source code
    ///   - Vec<u8>: Owned JavaScript source code as bytes
    ///
    /// # Example
    /// ```rust
    /// // From string literal
    /// let source = Source::from_bytes("let x = 1;");
    ///
    /// // From bytes
    /// let source = Source::from_bytes(b"let y = 2;");
    ///
    /// // From owned string
    /// let code = String::from("let z = 3;");
    /// let source = Source::from_bytes(&code);
    /// ```
    pub fn from_bytes<T: AsRef<[u8]>>(code: T) -> Self {
        Self {
            kind: SourceKind::JavaScript(code.as_ref().to_vec()),
            name: None,
        }
    }

    /// Set or change the name of the source
    ///
    /// # Example
    /// ```rust
    /// let source = Source::from_bytes("let x = 1;")
    ///     .with_name("example.js");
    /// ```
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Create a Source from compiled bytecode
    pub(crate) fn from_bytecode(code: impl Into<Vec<u8>>) -> Self {
        Self {
            kind: SourceKind::ByteCode(code.into()),
            name: None,
        }
    }

    /// Create a Source from a file path
    pub async fn from_path(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let code = tokio::fs::read(path.as_ref()).await?;
        let kind = match path.as_ref().extension().and_then(|ext| ext.to_str()) {
            Some("js") | Some("ts") | Some("mjs") => SourceKind::JavaScript(code),
            _ => SourceKind::ByteCode(code),
        };

        Ok(Self {
            kind,
            name: Some(path.as_ref().to_string_lossy().into_owned()),
        })
    }

    pub fn kind(&self) -> &SourceKind {
        &self.kind
    }

    /// Get the source bytes (either JavaScript code or bytecode)
    pub fn code(&self) -> &[u8] {
        match &self.kind {
            SourceKind::JavaScript(code) | SourceKind::ByteCode(code) => code,
        }
    }

    /// Get the source name/path if available
    ///
    /// # Returns
    /// - `Some(&str)` if the source was created using `from_path` and has a name/path
    /// - `None` if the source was created using `from_bytes` or `from_bytecode`
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the length of the source code in bytes
    pub fn len(&self) -> usize {
        self.code().len()
    }

    /// Returns true if the source is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
