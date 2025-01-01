use thiserror::Error;

#[derive(Error, PartialEq, Debug)]
pub enum RustyJSError {
    #[error("Failed to convert into type: {0}")]
    ConvertError(&'static str),

    #[error("Failed to borrow for type {0}")]
    Borrow(&'static str),

    #[error("Property Not Found")]
    PropertyNotFound,

    #[error("Not an Object")]
    NotObject,

    #[error("Not JS Function")]
    NotJSFunc,

    #[error("This has already been taken")]
    AlreadyTaken,

    #[error("{0}")]
    Eval(String),
}
