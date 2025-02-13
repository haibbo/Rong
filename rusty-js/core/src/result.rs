use crate::{
    FromJSValue, IntoJSValue, JSContext, JSError, JSException, JSExceptionHandler, JSObject,
    JSObjectOps, JSValueImpl,
};
use thiserror::Error;
use tokio::sync::oneshot;

pub type JSResult<T> = Result<T, RustyJSError>;

#[derive(Error, Debug)]
pub enum RustyJSError {
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

    #[error("Not JS Array")]
    NotJSArray,

    #[error("Not JS ArrayBuffer")]
    NotJSArrayBuffer,

    #[error("Not JS TypedArray")]
    NotJSTypedArray,

    #[error("Invalid TypedArray alignment: byte_offset must be a multiple of element size")]
    TypedArrayAlignmentError,

    #[error("Invalid TypedArray range: offset or length exceeds buffer size")]
    TypedArrayRangeError,

    #[error("Not JS Exception Object")]
    NotJSExcep,

    #[error("Failed to compile JS code to bytecode")]
    CompileToByteErr,

    #[error("Does not support bytecode")]
    NotSupportByteCode,

    #[error("{0}")]
    Error(String),

    #[error("{0}")]
    TypeError(String),

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
            RustyJSError::TypeError(_)
            | RustyJSError::NotJSFunc
            | RustyJSError::NotJSArray
            | RustyJSError::NotJSArrayBuffer
            | RustyJSError::NotJSTypedArray
            | RustyJSError::TypedArrayAlignmentError
            | RustyJSError::NotObject
            | RustyJSError::NotJSExcep
            | RustyJSError::InvalidParameter { .. } => {
                ctx.as_ref().throw_type_error(self.to_string())
            }

            RustyJSError::PropertyNotFound => ctx.as_ref().throw_reference_error(self.to_string()),

            RustyJSError::TypedArrayRangeError => ctx.as_ref().throw_range_error(self.to_string()),

            RustyJSError::Exception(_)
            | RustyJSError::Error(_)
            | RustyJSError::Borrow(_)
            | RustyJSError::CompileToByteErr
            | RustyJSError::NotSupportByteCode => ctx.as_ref().throw_error(self.to_string()),
        }
    }

    pub fn into_js_error<V>(self, ctx: &JSContext<V::Context>) -> JSObject<V>
    where
        V: JSObjectOps + JSValueImpl,
        V::Context: JSExceptionHandler,
    {
        // Quickjs does not support to receive exception on promise,  but jsc works.
        // here, regradless of error type, new general error to return
        let v = ctx.as_ref().new_error();
        let obj = JSObject::from_js_value(ctx, v).unwrap();
        let _ = obj.set("message", self.to_string());
        obj
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
        self.throw_js_exception(ctx)
    }
}

impl<V, T> IntoJSValue<V> for JSResult<T>
where
    V: JSObjectOps,
    V::Context: JSExceptionHandler,
    T: IntoJSValue<V>,
{
    fn into_js_value(self, ctx: &JSContext<V::Context>) -> V {
        match self {
            Ok(value) => value.into_js_value(ctx),
            Err(err) => err.into_js_value(ctx),
        }
    }
}

impl From<oneshot::error::RecvError> for RustyJSError {
    fn from(err: oneshot::error::RecvError) -> Self {
        RustyJSError::Error(format!("Tokio oneshot error: {}", err))
    }
}
