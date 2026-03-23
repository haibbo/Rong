use crate::formdata::FormData;
use bytes::Bytes;
use flate2::read::GzDecoder;
use http::HeaderMap;
use rong::*;
use rong_buffer::{Blob, File};
use rong_url::URLSearchParams;
use std::io::Read;
use std::sync::{Arc, Mutex};

type BodyChunk = Result<Bytes, String>;
type BodyReceiver = tokio::sync::mpsc::Receiver<BodyChunk>;
type SharedBodyReceiver = Arc<Mutex<Option<BodyReceiver>>>;

pub(crate) enum BodyKind {
    // Buffered, in-memory body as Bytes (Arc-backed, cheap clone, no aliasing issues)
    Buffered(Bytes),
    // Stream body via channel from net service (chunk or error)
    Channel(SharedBodyReceiver),
    JS(HttpBody),
}

// TODO: handle incoming well
impl Clone for BodyKind {
    fn clone(&self) -> Self {
        match self {
            Self::Buffered(b) => Self::Buffered(b.clone()),
            Self::Channel(rx) => Self::Channel(rx.clone()),
            Self::JS(arg0) => Self::JS(arg0.clone()),
        }
    }
}

#[derive(Clone)]
pub(crate) struct HttpBody(pub JSValue);

impl HttpBody {
    // Convert to bytes synchronously for hyper Body implementation
    pub async fn to_bytes(&self) -> JSResult<(Bytes, Option<String>)> {
        if let Some(obj) = self.0.clone().into_object() {
            let ctx = obj.get_ctx();

            // Handle URLSearchParams
            if let Ok(params) = obj.borrow::<URLSearchParams>() {
                return Ok((Bytes::from(params.to_string()), None));
            }

            // Handle TypedArray
            if let Some(typed_array) = AnyJSTypedArray::from_object(obj.clone())
                && let Some(bytes) = typed_array.as_bytes()
            {
                return Ok((Bytes::from(bytes.to_vec()), None));
            }

            // Handle ArrayBuffer
            if let Some(buffer) = JSArrayBuffer::from_object(obj.clone()) {
                return Ok((Bytes::from(buffer.as_bytes().to_vec()), None));
            }

            // Handle Blob
            if let Ok(blob) = obj.borrow::<Blob>() {
                return Ok((blob.bytes_ref().clone(), None));
            }

            // Handle File
            if let Ok(file) = obj.borrow::<File>() {
                return Ok((file.bytes_ref().clone(), None));
            }

            // Handle FormData
            let formdata = if let Ok(formdata) = obj.borrow::<FormData>() {
                Some(formdata.clone())
            } else {
                None
            };
            if let Some(formdata) = formdata {
                let (body, boundary) = formdata.serialize(ctx.clone()).await?;
                return Ok((Bytes::from(body), Some(boundary)));
            }

            // Handle other as empty string
            return Ok((Bytes::new(), None));
        }

        // Handle string
        if let Ok(s) = self.0.clone().try_into::<String>() {
            return Ok((Bytes::from(s), None));
        }

        Ok((Bytes::new(), None))
    }

    // Convert body to text
    pub async fn text(&self) -> JSResult<String> {
        // For most cases, we can just convert bytes to UTF-8 string
        let (bytes, _) = self.to_bytes().await?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }

    // Convert body to bytes
    pub async fn bytes(&self) -> JSResult<Bytes> {
        Ok(self.to_bytes().await?.0)
    }
}

// Decompress bytes based on content-encoding header
pub(crate) fn decompress_bytes(bytes: Bytes, headers: &HeaderMap) -> JSResult<Bytes> {
    if let Some(encoding) = headers.get(http::header::CONTENT_ENCODING) {
        match encoding.to_str() {
            Ok("gzip") => {
                let mut decoder = GzDecoder::new(&bytes[..]);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed).map_err(|e| {
                    HostError::new(
                        rong::error::E_IO,
                        format!("Failed to decompress gzip: {}", e),
                    )
                })?;
                Ok(Bytes::from(decompressed))
            }
            Ok(encoding) => Err(HostError::new(
                rong::error::E_NOT_SUPPORTED,
                format!("Unsupported content-encoding: {}", encoding),
            )
            .into()),
            Err(_) => Ok(bytes),
        }
    } else {
        Ok(bytes)
    }
}
