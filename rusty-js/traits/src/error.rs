use std::fmt;

pub trait ExtractJSError {
    fn extract_err_msg(&self) -> String;
}

#[derive(Debug)]
pub struct JSError {
    message: String,
}

impl JSError {
    pub fn new<V: ExtractJSError>(v: &V) -> Self {
        Self {
            message: v.extract_err_msg(),
        }
    }
}

impl fmt::Display for JSError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for JSError {}
