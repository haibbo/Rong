use crate::{
    FromJSValue, IntoJSValue, JSContext, JSError, JSException, JSExceptionHandler, JSObject,
    JSObjectOps, JSValueImpl,
};
use thiserror::Error;
use tokio::sync::oneshot;

pub type JSResult<T> = Result<T, RustyJSError>;

#[derive(Error, Debug)]
pub enum RustyJSError {
    #[error("Failed to convert from {0} to {1}")]
    ConvertError(&'static str, &'static str),

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

    #[error("Failed to compile JS code to bytecode")]
    CompileToByteErr,

    #[error("{0}")]
    Error(String),

    #[error("{0}")]
    Exception(#[from] JSError),
}

impl RustyJSError {
    pub fn throw_js_exception<V>(self, ctx: &JSContext<V::Context>) -> V
    where
        V: JSValueImpl,
        V::Context: JSExceptionHandler,
    {
        match self {
            RustyJSError::ConvertError(_, _)
            | RustyJSError::NotJSFunc
            | RustyJSError::NotObject
            | RustyJSError::NotJSExcep
            | RustyJSError::InvalidParameter { .. } => {
                ctx.as_ref().throw_type_error(self.to_string())
            }

            RustyJSError::PropertyNotFound => ctx.as_ref().throw_reference_error(self.to_string()),

            RustyJSError::Exception(_)
            | RustyJSError::Error(_)
            | RustyJSError::Borrow(_)
            | RustyJSError::CompileToByteErr
            | RustyJSError::AlreadyTaken => ctx.as_ref().throw_error(self.to_string()),
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
    fn from_js_value(ctx: &JSContext<V::Context>, value: V) -> JSResult<Self> {
        let obj = JSObject::from_js_value(ctx, value)?;
        Ok(RustyJSError::Exception(
            JSException::from_object(obj).into_error(),
        ))
    }
}

impl<V: JSValueImpl> IntoJSValue<V> for RustyJSError
where
    V::Context: JSExceptionHandler,
    V: JSObjectOps,
{
    fn into_js_value(self, ctx: &JSContext<V::Context>) -> V {
        let v = ctx.as_ref().new_error();
        let obj = JSObject::from_js_value(ctx, v).unwrap();
        obj.set("message", self.to_string());
        obj.into_inner()
    }
}

impl From<oneshot::error::RecvError> for RustyJSError {
    fn from(err: oneshot::error::RecvError) -> Self {
        RustyJSError::Error(format!("Tokio oneshot error: {}", err))
    }
}
