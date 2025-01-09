use crate::{
    function::JSParameterType, FromJSValue, JSContext, JSError, JSException, JSExceptionHandler,
    JSObject, JSObjectOps, JSValueImpl,
};
use thiserror::Error;

pub type JSResult<T> = Result<T, RustyJSError>;

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

    pub fn into_js_error<V>(self, ctx: &JSContext<V::Context>) -> JSException<V>
    where
        V: JSValueImpl + JSObjectOps,
        V::Context: JSExceptionHandler,
    {
        ctx.new_js_error(&self.to_string())
    }
}

impl<V: JSValueImpl> FromJSValue<V> for RustyJSError
where
    V: JSObjectOps,
{
    fn from_js_value(ctx: &V::Context, value: V) -> JSResult<Self> {
        let obj = JSObject::from_js_value(ctx, value)?;
        Ok(RustyJSError::Exception(
            JSException::from_object(obj).into_error(),
        ))
    }
}

// blanket implementing.
impl JSParameterType for RustyJSError {}
