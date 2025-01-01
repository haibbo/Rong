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
