use super::blob::Blob;
use rusty_js::{function::Optional, *};
use std::time::{SystemTime, UNIX_EPOCH};

#[js_class]
pub struct File {
    blob: Blob,
    filename: String,
    last_modified: i64,
}

#[js_methods]
impl File {
    #[js_method(constructor)]
    fn new(data: JSArray, filename: String, options: Optional<JSObject>) -> JSResult<Self> {
        let mut last_modified = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| RustyJSError::Error("Failed to get current time".to_string()))?
            .as_millis() as i64;

        if let Some(ref opts) = options.0 {
            last_modified = opts.get::<_, i64>("lastModified")?;
        }

        let blob = Blob::new(Optional(Some(data)), options)?;

        Ok(Self {
            blob,
            filename,
            last_modified,
        })
    }

    #[js_method(get)]
    pub fn size(&self) -> usize {
        self.blob.size()
    }

    #[js_method(get)]
    pub fn name(&self) -> String {
        self.filename.clone()
    }

    #[js_method(get, rename = "type")]
    pub fn mime_type(&self) -> String {
        self.blob.mime_type()
    }

    #[js_method(get, rename = "lastModified")]
    pub fn last_modified(&self) -> i64 {
        self.last_modified
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

pub(crate) fn init(ctx: &JSContext) -> JSResult<()> {
    ctx.register_class::<File>();
    Ok(())
}
