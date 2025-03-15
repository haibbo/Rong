use super::blob::Blob;
use rusty_js::{function::Optional, *};
use std::time::{SystemTime, UNIX_EPOCH};

#[js_export]
pub struct File {
    blob: Blob,
    filename: String,
    last_modified: i64,
}

#[js_class]
impl File {
    #[js_method(constructor)]
    fn new(data: JSArray, filename: String, options: Optional<JSObject>) -> JSResult<Self> {
        // Validate filename
        if filename.is_empty() {
            return Err(RustyJSError::TypeError(
                "File name cannot be empty".to_string(),
            ));
        }

        // Get current time as default last_modified
        let default_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| RustyJSError::Error("Failed to get current time".to_string()))?
            .as_millis() as i64;

        // Extract options
        let (last_modified, blob_options) = if let Some(ref opts) = options.0 {
            let time = match opts.get::<_, f64>("lastModified") {
                Ok(t) => t as i64,
                Err(_) => default_time,
            };
            (time, options)
        } else {
            (default_time, Optional(None))
        };

        let blob = Blob::new(Optional(Some(data)), blob_options)?;

        Ok(Self {
            blob,
            filename,
            last_modified,
        })
    }

    #[js_method(getter)]
    pub fn size(&self) -> usize {
        self.blob.size()
    }

    #[js_method(getter)]
    pub fn name(&self) -> String {
        self.filename.clone()
    }

    #[js_method(getter, rename = "type")]
    pub fn mime_type(&self) -> String {
        self.blob.mime_type()
    }

    #[js_method(getter, rename = "lastModified")]
    pub fn last_modified(&self) -> f64 {
        self.last_modified as f64
    }

    #[js_method]
    pub fn slice(
        &self,
        start: Optional<i64>,
        end: Optional<i64>,
        content_type: Optional<String>,
    ) -> JSResult<Blob> {
        self.blob.slice(start, end, content_type)
    }

    #[js_method]
    pub async fn text(&mut self) -> JSResult<String> {
        self.blob.text().await
    }

    #[js_method(rename = "arrayBuffer")]
    pub async fn array_buffer(&self, ctx: JSContext) -> JSResult<JSArrayBuffer<u8>> {
        self.blob.array_buffer(ctx).await
    }

    #[js_method]
    pub async fn bytes(&self, ctx: JSContext) -> JSResult<JSTypedArray> {
        self.blob.bytes(ctx).await
    }
}
