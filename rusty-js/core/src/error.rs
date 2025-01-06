use crate::{JSError, JSExceptionHandler, JSValueImpl};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RustyJSError {
    #[error("Failed to convert into type: {0}")]
    ConvertError(&'static str),

    #[error("Failed to borrow for type {0}")]
    Borrow(&'static str),

    #[error("invalid parameters, expected {expected} arguments, got {got}")]
    InvalidParameter { expected: u32, got: u32 },

    #[error("Property Not Found")]
    PropertyNotFound,

    #[error("Not an JS Object")]
    NotObject,

    #[error("Not JS Function")]
    NotJSFunc,

    #[error("Not JS Exception Object")]
    NotJSExcep,

    #[error("This has already been taken")]
    AlreadyTaken,

    #[error("{0}")]
    Error(String),

    #[error("{0}")]
    Exception(#[from] JSError),
}

impl RustyJSError {
    pub fn throw_js_exception<V>(self, ctx: &V::Context) -> V
    where
        V: JSValueImpl,
        V::Context: JSExceptionHandler,
    {
        match self {
            RustyJSError::ConvertError(_)
            | RustyJSError::NotJSFunc
            | RustyJSError::NotObject
            | RustyJSError::NotJSExcep
            | RustyJSError::InvalidParameter { .. } => ctx.throw_type_error(self.to_string()),

            RustyJSError::PropertyNotFound => ctx.throw_reference_error(self.to_string()),

            RustyJSError::Exception(_)
            | RustyJSError::Error(_)
            | RustyJSError::Borrow(_)
            | RustyJSError::AlreadyTaken => ctx.throw_error(self.to_string()),
        }
    }
}
