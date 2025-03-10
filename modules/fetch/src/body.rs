use bytes::Bytes;
use hyper::body::Incoming;
use lxr_url::URLSearchParams;
use rusty_js::*;
use std::{
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

#[derive(Clone)]
pub(crate) enum BodyKind {
    Hyper(Rc<Incoming>), // need Rc for Clone support
    JS(HttpBody),
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
    fn to_bytes(&self) -> Result<Bytes, RustyJSError> {
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

impl hyper::body::Body for HttpBody {
    type Data = Bytes;
    type Error = RustyJSError;

    fn poll_frame(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>> {
        // Convert to bytes synchronously
        match self.get_mut().to_bytes() {
            Ok(bytes) => Poll::Ready(Some(Ok(hyper::body::Frame::data(bytes)))),
            Err(e) => Poll::Ready(Some(Err(e))),
        }
    }
}
