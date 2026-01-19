use crate::{
    FromJSValue, IntoJSValue, JSContext, JSContextImpl, JSExceptionHandler, JSObjectOps, JSValue,
    JSValueImpl,
};
use thiserror::Error;
use tokio::sync::oneshot;

pub type JSResult<T> = Result<T, RongJSError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThrownValueHandle {
    pub(crate) context_id: usize,
    pub(crate) id: u32,
    pub(crate) generation: u32,
}

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

    /// Thrown/rejected JS payload (can be any JS value, including primitives).
    #[error("JavaScript threw a value")]
    Thrown(ThrownValueHandle),
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

            RongJSError::Error(_)
            | RongJSError::Borrow(_)
            | RongJSError::CompileToByteErr
            | RongJSError::OnceFnCalled
            | RongJSError::NotSupportByteCode => ctx.as_ref().throw_error(self.to_string()),

            RongJSError::Thrown(handle) => {
                let Some(value) = ctx.take_thrown(handle) else {
                    return ctx.as_ref().throw_error("Invalid thrown value handle");
                };
                ctx.throw(value).into_value()
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
        let ctx: JSContext<V::Context> =
            JSContext::from_borrowed_raw_ptr(value.as_value().as_raw_context());
        RongJSError::Thrown(ctx.capture_thrown(value))
    }

    pub fn thrown_value<C>(&self, ctx: &JSContext<C>) -> Option<JSValue<C::Value>>
    where
        C: JSContextImpl,
    {
        match *self {
            RongJSError::Thrown(handle) => ctx.resolve_thrown(handle),
            _ => None,
        }
    }
}

impl<V: JSValueImpl> FromJSValue<V> for RongJSError
where
    V: JSObjectOps,
{
    fn from_js_value(ctx: &JSContext<V::Context>, value: V) -> JSResult<Self> {
        Ok(RongJSError::from_jsvalue(JSValue::from_raw(ctx, value)))
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
