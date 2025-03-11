use bytes::Bytes;
use flate2::read::GzDecoder;
use http::HeaderMap;
use hyper::body::Incoming;
use lxr_url::URLSearchParams;
use rusty_js::*;
use std::io::Read;

pub(crate) enum BodyKind {
    Hyper(Option<Incoming>),
    JS(HttpBody),
}

// TODO: handle incoming well
impl Clone for BodyKind {
    fn clone(&self) -> Self {
        match self {
            Self::Hyper(_) => Self::Hyper(None),
            Self::JS(arg0) => Self::JS(arg0.clone()),
        }
    }
}

#[derive(Clone)]
pub(crate) struct HttpBody(pub JSValue);

impl HttpBody {
    // Convert body to text
    pub async fn text(&self) -> JSResult<String> {
        if let Ok(s) = self.0.clone().try_into::<String>() {
            return Ok(s);
        }

        if let Some(obj) = self.0.clone().into_object() {
            // Handle URLSearchParams
            if let Ok(params) = obj.borrow::<URLSearchParams>() {
                return Ok(params.to_string());
            }

            // Handle TypedArray/ArrayBuffer
            if obj.is_array_buffer() {
                let array = JSArrayBuffer::<u8>::from_object(obj).ok_or_else(|| {
                    RustyJSError::TypeError("Failed to convert ArrayBuffer".to_string())
                })?;
                return Ok(String::from_utf8_lossy(&array.to_vec()).into_owned());
            }

            // Handle other objects by converting to string
            return Ok(obj.to_string());
        }

        Ok(String::new())
    }

    // Convert body to bytes
    pub async fn bytes(&self) -> JSResult<Bytes> {
        self.to_bytes()
    }

    // Convert to bytes synchronously for hyper Body implementation
    pub fn to_bytes(&self) -> JSResult<Bytes> {
        if let Ok(s) = self.0.clone().try_into::<String>() {
            return Ok(Bytes::from(s));
        }

        if let Some(obj) = self.0.clone().into_object() {
            // Handle URLSearchParams
            if let Ok(params) = obj.borrow::<URLSearchParams>() {
                return Ok(Bytes::from(params.to_string()));
            }

            // Handle TypedArray/ArrayBuffer
            if obj.is_array_buffer() {
                let array = JSArrayBuffer::<u8>::from_object(obj).ok_or_else(|| {
                    RustyJSError::TypeError("Failed to convert ArrayBuffer".to_string())
                })?;
                return Ok(Bytes::from(array.to_vec()));
            }

            // Handle other as empty bytes
            return Ok(Bytes::new());
        }

        Ok(Bytes::new())
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
                    RustyJSError::Error(format!("Failed to decompress gzip: {}", e))
                })?;
                Ok(Bytes::from(decompressed))
            }
            Ok(encoding) => Err(RustyJSError::Error(format!(
                "Unsupported content-encoding: {}",
                encoding
            ))),
            Err(_) => Ok(bytes),
        }
    } else {
        Ok(bytes)
    }
}
