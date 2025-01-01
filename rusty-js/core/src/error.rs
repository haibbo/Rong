use crate::{JSExceptionHandler, JSValueImpl};
use thiserror::Error;

#[derive(Error, PartialEq, Debug)]
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

    #[error("This has already been taken")]
    AlreadyTaken,

    #[error("{0}")]
    Error(String),
}

impl RustyJSError {
    pub fn into_js_exception<V>(self, ctx: &V::Context) -> V
    where
        V: JSValueImpl,
        V::Context: JSExceptionHandler,
    {
        match self {
            RustyJSError::ConvertError(_)
            | RustyJSError::NotJSFunc
            | RustyJSError::NotObject
            | RustyJSError::InvalidParameter { .. } => ctx.throw_type_error(self.to_string()),

            RustyJSError::PropertyNotFound => ctx.throw_reference_error(self.to_string()),

            RustyJSError::Borrow(_) | RustyJSError::AlreadyTaken | RustyJSError::Error(_) => {
                ctx.throw_error(self.to_string())
            }
        }
    }
}
