use crate::{
    FromJSValue, IntoJSValue, JSContext, JSContextImpl, JSObject, JSObjectOps, JSResult, JSTypeOf,
    JSValue, JSValueImpl, RustyJSError,
};
use std::fmt;
use std::ops::Deref;
use std::string::String;

/// Represents a JavaScript exception object wrapper
pub struct JSException<V: JSValueImpl>(JSObject<V>);

impl<V: JSValueImpl> Deref for JSException<V> {
    type Target = JSObject<V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<V> IntoJSValue<V> for JSException<V>
where
    V: JSValueImpl,
{
    fn into_js_value(self, ctx: &V::Context) -> V {
        self.0.into_js_value(ctx)
    }
}

impl<V> JSException<V>
where
    V: JSValueImpl,
{
    pub(crate) fn from_object(v: JSObject<V>) -> Self {
        Self(v)
    }
}

impl<V> FromJSValue<V> for JSException<V>
where
    V: JSTypeOf,
{
    fn from_js_value(ctx: &V::Context, value: V) -> JSResult<Self> {
        value
            .is_exception()
            .map(|v| Self(JSValue::from_raw_parts(ctx.clone(), v).into()))
            .ok_or(RustyJSError::NotObject)
    }
}

impl<C: JSContextImpl> JSContext<C> {
    pub fn new_js_error(&self, message: &str) -> JSException<C::Value>
    where
        C: JSExceptionHandler,
        C::Value: JSObjectOps,
    {
        let v = self.as_ref().new_error();
        let obj = JSObject::from_js_value(self.as_ref(), v).unwrap();
        obj.set("message", message);
        JSException::from_object(obj)
    }
}

impl<V> JSException<V>
where
    V: JSObjectOps,
{
    /// Returns the message of the error.
    ///
    /// Same as retrieving `error.message` in JavaScript.
    pub fn message(&self) -> Option<String> {
        self.get("message").ok()
    }

    /// Returns the stack of the error.
    ///
    /// Same as retrieving `error.stack` in JavaScript.
    pub fn stack(&self) -> Option<String> {
        self.get("stack").ok()
    }

    /// Convert the exception into JSError
    pub fn into_error(self) -> JSError {
        let ctx = self.as_ctx().clone();
        if self.is_error().is_some() {
            JSError {
                message: self.message(),
                stack: self.stack(),
            }
        } else {
            let js_value = self.into_js_value(&ctx);
            JSError {
                message: String::from_js_value(&ctx, js_value).ok(),
                stack: None,
            }
        }
    }
}

/// Represents a JavaScript error with message and stack trace
#[derive(Debug)]
pub struct JSError {
    pub message: Option<String>,
    pub stack: Option<String>,
}

impl fmt::Display for JSError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        match (&self.message, &self.stack) {
            (Some(msg), Some(stack)) => write!(f, "{}\n{}", msg, stack),
            (Some(msg), None) => write!(f, "{}", msg),
            (None, Some(stack)) => write!(f, "{}", stack),
            (None, None) => write!(f, "Unknown JavaScript Error"),
        }
    }
}

impl std::error::Error for JSError {}

pub trait JSExceptionHandler: JSContextImpl {
    fn throw_syntax_error(&self, message: impl AsRef<str>) -> Self::Value;
    fn throw_type_error(&self, message: impl AsRef<str>) -> Self::Value;
    fn throw_reference_error(&self, message: impl AsRef<str>) -> Self::Value;
    fn throw_range_error(&self, message: impl AsRef<str>) -> Self::Value;
    fn throw_error(&self, message: impl AsRef<str>) -> Self::Value;
    fn new_error(&self) -> Self::Value;
}

impl<C> JSContext<C>
where
    C: JSContextImpl + JSExceptionHandler,
    C::Value: JSValueImpl,
{
    pub fn throw_syntax_error(&self, message: impl AsRef<str>) -> JSValue<C::Value> {
        let raw = self.as_ref().throw_syntax_error(message);
        JSValue::new(self, raw)
    }

    pub fn throw_type_error(&self, message: impl AsRef<str>) -> JSValue<C::Value> {
        let raw = self.as_ref().throw_type_error(message);
        JSValue::new(self, raw)
    }

    pub fn throw_reference_error(&self, message: impl AsRef<str>) -> JSValue<C::Value> {
        let raw = self.as_ref().throw_reference_error(message);
        JSValue::new(self, raw)
    }

    pub fn throw_range_error(&self, message: impl AsRef<str>) -> JSValue<C::Value> {
        let raw = self.as_ref().throw_range_error(message);
        JSValue::new(self, raw)
    }

    pub fn throw_error(&self, message: impl AsRef<str>) -> JSValue<C::Value> {
        let raw = self.as_ref().throw_error(message);
        JSValue::new(self, raw)
    }
}

impl<V: JSObjectOps> fmt::Debug for JSException<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Exception")
            .field("message", &self.message())
            .field("stack", &self.stack())
            .finish()
    }
}

impl<V: JSObjectOps> fmt::Display for JSException<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_error().is_some() {
            "Error:".fmt(f)?;
            if let Some(message) = &self.message() {
                ' '.fmt(f)?;
                message.fmt(f)?;
            }
            if let Some(stack) = &self.stack() {
                '\n'.fmt(f)?;
                stack.fmt(f)?;
            }
        } else {
            let ctx = self.as_ctx();
            let js_value = self.as_inner().clone();
            String::from_js_value(ctx, js_value).unwrap().fmt(f)?;
        }
        Ok(())
    }
}

// blanket implementing.
impl<V: JSValueImpl> crate::function::JSParameterType for JSException<V> {}
