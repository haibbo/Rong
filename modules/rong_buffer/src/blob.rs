use rong::{function::Optional, js_class, js_export, js_method, *};

#[derive(Default)]
struct BlobOptions {
    type_: String,
    endings: EndingType,
}

#[derive(Default)]
enum EndingType {
    #[default]
    Transparent,
    Native,
}

#[cfg(windows)]
const LINE_ENDING: &[u8] = b"\r\n";
#[cfg(not(windows))]
const LINE_ENDING: &[u8] = b"\n";

#[js_export]
pub struct Blob {
    mime_type: String,
    data: Vec<u8>,
}

#[js_class]
impl Blob {
    #[js_method(constructor)]
    pub fn new(parts: Optional<JSArray>, options: Optional<JSObject>) -> JSResult<Self> {
        let mut blob_data = Vec::new();
        let mut blob_options = BlobOptions::default();

        // Parse options if provided
        if let Some(opts) = options.0 {
            if let Ok(type_) = opts.get::<_, String>("type") {
                blob_options.type_ = normalize_type(type_);
            }
            if let Ok(endings) = opts.get::<_, String>("endings") {
                blob_options.endings = match endings.as_str() {
                    "native" => EndingType::Native,
                    "transparent" => EndingType::Transparent,
                    _ => EndingType::Transparent,
                };
            }
        }

        // Process parts if provided
        if let Some(parts) = parts.0 {
            blob_data = process_blob_part(&parts, &blob_options).map_err(|e| {
                HostError::new(
                    rong::error::E_INVALID_ARG,
                    format!("Failed to process blob parts: {}", e),
                )
                .with_name("TypeError")
            })?;
        }

        Ok(Self::from_parts(blob_options.type_, blob_data))
    }

    pub fn from_parts(mime: String, data: Vec<u8>) -> Self {
        Self {
            mime_type: mime,
            data,
        }
    }

    #[js_method(getter, enumerable)]
    pub fn size(&self) -> usize {
        self.data.len()
    }

    #[js_method(getter, enumerable, rename = "type")]
    pub fn mime_type(&self) -> String {
        self.mime_type.clone()
    }

    /// Returns a promise that resolves with an ArrayBuffer containing the blob's data
    #[js_method(rename = "arrayBuffer")]
    pub async fn array_buffer(&self, ctx: JSContext) -> JSResult<JSArrayBuffer<u8>> {
        JSArrayBuffer::from_bytes(&ctx, &self.data)
    }

    /// Returns a promise that resolves with a text representation of the blob's data
    #[js_method]
    pub async fn text(&self) -> JSResult<String> {
        String::from_utf8(self.data.clone()).map_err(|e| {
            HostError::new(
                rong::error::E_INVALID_DATA,
                format!("Invalid UTF-8 sequence: {}", e),
            )
            .into()
        })
    }

    /// Returns a new Blob containing a subset of this blob's data
    ///
    /// # Arguments
    /// * `start` - The starting index where to start copying from
    /// * `end` - Optional ending index where to end copying (exclusive)
    /// * `content_type` - Optional new content type for the new blob
    #[js_method]
    pub fn slice(
        &self,
        start: Optional<i64>,
        end: Optional<i64>,
        content_type: Optional<String>,
    ) -> JSResult<Self> {
        let len = self.data.len() as i64;

        // Convert negative indices to positive
        let start = start.0.unwrap_or(0);
        let end = end.0.unwrap_or(len);

        let start = if start < 0 {
            (len + start).max(0)
        } else {
            start.min(len)
        };
        let end = if end < 0 {
            (len + end).max(0)
        } else {
            end.min(len)
        };

        // Convert to usize after bounds checking
        let start = start as usize;
        let end = end as usize;

        // Handle invalid ranges
        if start > end {
            return Ok(Self {
                mime_type: content_type.0.unwrap_or_default(),
                data: Vec::new(),
            });
        }

        Ok(Self {
            mime_type: content_type.0.unwrap_or_else(|| self.mime_type.clone()),
            data: self.data[start..end].to_vec(),
        })
    }

    /// Returns a promise that resolves with a Uint8Array containing the blob's data
    #[js_method]
    pub async fn bytes(&self, ctx: JSContext) -> JSResult<JSTypedArray> {
        let buffer = JSArrayBuffer::from_bytes(&ctx, &self.data)?;
        JSTypedArray::from_array_buffer::<u8>(&ctx, buffer, 0, None)
    }
}

/// Process a single Blob part according to the specification
///
/// This function processes various types of input data into a byte vector:
/// - Blob objects: copies their internal data
/// - ArrayBuffer: copies the buffer contents
/// - TypedArray: copies the array contents
/// - String: converts to UTF-8 bytes with optional line ending normalization
///
/// # Arguments
///
/// * `array` - Array of items to process
/// * `options` - Blob options including MIME type and line ending preferences
///
/// # Returns
///
/// * `JSResult<Vec<u8>>` - Processed bytes or error if processing fails
fn process_blob_part(array: &JSArray, options: &BlobOptions) -> JSResult<Vec<u8>> {
    let mut data = Vec::new();

    if array.is_empty() {
        return Ok(data);
    }

    for elem in array.iter::<JSValue>() {
        let elem = elem?;

        if let Some(object) = elem.clone().into_object() {
            if let Some(typed_array) = JSTypedArray::from_object(object.clone()) {
                if let Some(bytes) = typed_array.as_bytes() {
                    data.extend_from_slice(bytes);
                }
                continue;
            }

            if let Some(buffer) = JSArrayBuffer::<u8>::from_object(object.clone()) {
                if let Some(bytes) = buffer.as_bytes() {
                    data.extend_from_slice(bytes);
                }
                continue;
            }

            if let Ok(blob) = object.borrow::<Blob>() {
                data.extend_from_slice(&blob.data);
                continue;
            }
        }

        if let Ok(string) = elem.try_into::<String>() {
            match options.endings {
                EndingType::Native => {
                    let mut chars = string.chars().peekable();
                    while let Some(c) = chars.next() {
                        match c {
                            '\r' => {
                                if chars.peek() == Some(&'\n') {
                                    chars.next(); // skip \n
                                }
                                data.extend_from_slice(LINE_ENDING);
                            }
                            '\n' => {
                                data.extend_from_slice(LINE_ENDING);
                            }
                            c => {
                                let mut buf = [0; 4];
                                data.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                            }
                        }
                    }
                }
                EndingType::Transparent => {
                    data.extend_from_slice(string.as_bytes());
                }
            }
            continue;
        }

        return Err(
            HostError::new(rong::error::E_INVALID_ARG, "Unsupported Blob part type")
                .with_name("TypeError")
                .into(),
        );
    }

    Ok(data)
}

/// Normalizes a MIME type string according to the Blob specification.
///
/// According to the specification:
/// 1. If the MIME type string contains any characters outside the range U+0020 to U+007E,
///    return an empty string and abort these steps.
/// 2. Convert the MIME type string to ASCII lowercase and return the result.
///
/// # Arguments
/// * `mime_type` - The MIME type string to normalize
///
/// # Returns
/// * A normalized MIME type string, or an empty string if the input contains invalid characters
fn normalize_type(mime_type: String) -> String {
    // Check for any characters outside the range U+0020 to U+007E
    if mime_type.chars().any(|c| !(' '..'~').contains(&c)) {
        return String::new();
    }

    // Convert to ASCII lowercase and return
    mime_type.to_ascii_lowercase()
}
