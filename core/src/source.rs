use std::path::Path;

use crate::{HostError, JSContext, JSContextImpl, JSResult};

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
    ///   - `Vec<u8>`: Owned JavaScript source code as bytes
    ///
    /// # Example
    /// ```rust
    /// use rong_core::Source;
    ///
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
    /// use rong_core::Source;
    ///
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

    pub async fn save_bytecode<C: JSContextImpl>(
        &self,
        ctx: &JSContext<C>,
        path: impl AsRef<Path>,
    ) -> JSResult<()> {
        use tokio::io::AsyncWriteExt;

        // Verify file extension
        if path.as_ref().extension().and_then(|ext| ext.to_str()) != Some("rong") {
            return Err(HostError::new(
                crate::error::E_INVALID_ARG,
                "Bytecode files must have .rong extension",
            )
            .with_name("TypeError")
            .into());
        }

        // Open file with explicit create and truncate options
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path.as_ref())
            .await?;

        // Write header with separator
        file.write_all(b"RONG").await?;
        file.write_all(ctx.runtime().engine.as_bytes()).await?;
        file.write_all(&[0]).await?; // Null separator

        // Write bytecode
        file.write_all(self.code()).await?;

        Ok(())
    }

    /// Load JavaScript source code from a file path asynchronously
    ///
    /// Detects file type (.js,.rong) and handles bytecode verification
    /// if a .rong file is loaded.
    ///
    /// # Returns
    /// * `Ok(Source)` - The loaded source code, ready for evaluation or compilation
    /// * `Err(RongJSError)` - If file reading fails, type is unsupported, or bytecode is invalid
    pub async fn from_path<C: JSContextImpl>(
        ctx: &JSContext<C>,
        path: impl AsRef<Path>,
    ) -> JSResult<Self> {
        let code = tokio::fs::read(path.as_ref()).await?;

        let kind = match path.as_ref().extension().and_then(|ext| ext.to_str()) {
            Some("js") => SourceKind::JavaScript(code),
            Some("rong") => {
                // Verify bytecode header
                if code.len() >= 6 && &code[0..4] == b"RONG" {
                    let engine_name = ctx.runtime().engine.to_string();
                    let expected_header = format!("RONG{}", engine_name);

                    if code.len() > expected_header.len()
                        && &code[0..expected_header.len()] == expected_header.as_bytes()
                        && code[expected_header.len()] == 0
                    {
                        // Skip header and null separator
                        SourceKind::ByteCode(code[expected_header.len() + 1..].to_vec())
                    } else {
                        return Err(
                            HostError::new(
                                crate::error::E_NOT_SUPPORTED,
                                format!(
                                    "Bytecode was compiled for a different engine. Expected: {}, Found: {}",
                                    engine_name,
                                    String::from_utf8_lossy(&code[6..])
                                ),
                            )
                            .into(),
                        );
                    }
                } else {
                    return Err(HostError::new(
                        crate::error::E_INVALID_DATA,
                        "Invalid .rong file format",
                    )
                    .into());
                }
            }
            _ => {
                return Err(HostError::new(
                    crate::error::E_NOT_SUPPORTED,
                    format!(
                        "Unsupported file type. Supported extensions: .js,.rong. Found: {}",
                        path.as_ref().display()
                    ),
                )
                .into());
            }
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
