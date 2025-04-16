use crate::{
    FromJSValue, IntoJSValue, JSContext, JSError, JSException, JSExceptionHandler, JSObject,
    JSObjectOps, JSValue, JSValueImpl,
};
use thiserror::Error;
use tokio::sync::oneshot;

pub type JSResult<T> = Result<T, RongJSError>;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum RongJSError {
    #[error("Failed to borrow for type {0}")]
    Borrow(&'static str),

    #[error("{expected} arguments required, but {got} found")]
    InvalidParameter { expected: u32, got: u32 },

    #[error("Property '{0}' Not Found")]
    PropertyNotFound(String),

    #[error("Not JS Object")]
    NotObject,

    #[error("Not JS Symbol")]
    NotSymbol,

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

    #[error("OnceFn had been called")]
    OnceFnCalled,

    #[error("{0}")]
    Error(String),

    #[error("{0}")]
    TypeError(String),

    #[error("{0}")]
    Exception(#[from] JSError),

    #[error("{0}")]
    JSValue(#[from] JSValueErr),
}

impl RongJSError {
    pub fn throw_js_exception<V>(self, ctx: &JSContext<V::Context>) -> V
    where
        V: JSValueImpl,
        V::Context: JSExceptionHandler,
    {
        match self {
            RongJSError::TypeError(_)
            | RongJSError::NotJSFunc
            | RongJSError::NotJSArray
            | RongJSError::NotJSArrayBuffer
            | RongJSError::NotJSTypedArray
            | RongJSError::TypedArrayAlignmentError
            | RongJSError::NotObject
            | RongJSError::NotSymbol
            | RongJSError::NotJSExcep
            | RongJSError::InvalidParameter { .. } => {
                ctx.as_ref().throw_type_error(self.to_string())
            }

            RongJSError::PropertyNotFound(_) => {
                ctx.as_ref().throw_reference_error(self.to_string())
            }

            RongJSError::TypedArrayRangeError => ctx.as_ref().throw_range_error(self.to_string()),

            RongJSError::Exception(_)
            | RongJSError::Error(_)
            | RongJSError::Borrow(_)
            | RongJSError::CompileToByteErr
            | RongJSError::OnceFnCalled
            | RongJSError::NotSupportByteCode => ctx.as_ref().throw_error(self.to_string()),

            RongJSError::JSValue(JSValueErr(value)) => {
                let value = unsafe { Box::from_raw(value as *mut JSValue<V>) };
                ctx.throw(*value).into_value()
            }
        }
    }

    pub fn into_js_error<V>(self, ctx: &JSContext<V::Context>) -> JSValue<V>
    where
        V: JSValueImpl,
        V::Context: JSExceptionHandler,
    {
        let v = self.throw_js_exception(ctx);
        JSValue::from_raw(ctx, v)
    }

    pub fn from_jsvalue<V: JSValueImpl>(value: JSValue<V>) -> Self {
        let addr = Box::new(value);
        RongJSError::JSValue(JSValueErr(Box::into_raw(addr) as usize))
    }
}

impl<V: JSValueImpl> FromJSValue<V> for RongJSError
where
    V: JSObjectOps,
{
    fn from_js_value(ctx: &JSContext<V::Context>, value: V) -> JSResult<Self> {
        let obj = JSObject::from_js_value(ctx, value)?;
        Ok(RongJSError::Exception(
            JSException::from_object(obj)
                .ok_or(RongJSError::NotObject)?
                .into_error(),
        ))
    }
}

impl<V: JSValueImpl> IntoJSValue<V> for RongJSError
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

impl From<oneshot::error::RecvError> for RongJSError {
    fn from(err: oneshot::error::RecvError) -> Self {
        RongJSError::Error(format!("Tokio oneshot error: {}", err))
    }
}

pub trait IntoJSResult<T> {
    fn into_result(self) -> JSResult<T>;
    fn into_type_result(self) -> JSResult<T>;
}

impl<T, E> IntoJSResult<T> for std::result::Result<T, E>
where
    E: ToString,
{
    fn into_result(self) -> JSResult<T> {
        self.map_err(|e| RongJSError::Error(e.to_string()))
    }

    fn into_type_result(self) -> JSResult<T> {
        self.map_err(|e| RongJSError::TypeError(e.to_string()))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct JSValueErr(usize);

impl std::fmt::Display for JSValueErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSValue Error")
    }
}

impl std::error::Error for JSValueErr {}
