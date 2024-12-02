use crate::{JSContext, JSContextKind, JSValue, JSValueKind};
use std::fmt;

/// extract exception and error details from JSValue
/// this trait can only be implemented by JSValue
pub trait JSValueError: JSValueKind
where
    Self: Sized,
    Self: TryInto<String, Error = String>,
{
    fn get_exception_message(value: &JSValue<Self>) -> Option<String> {
        let msg: Result<String, String> = value.clone().try_into();
        match msg {
            Ok(v) => Some(v),
            Err(v) => Some(v),
        }
    }

    fn get_exception_stack(_value: &JSValue<Self>) -> Option<String> {
        /* TODO: need object ready
        value
            .as_object()
            .and_then(|obj| obj.get_property("stack"))
            .and_then(|stack| stack.into_rust())
        */
        Some(String::from("TODO get stack"))
    }
}

impl<'ctx, V> JSValue<'ctx, V>
where
    V: JSValueError,
{
    pub fn into_error(self) -> JSErrorInfo {
        JSErrorInfo {
            message: V::get_exception_message(&self),
            stack: V::get_exception_stack(&self),
        }
    }
}

#[derive(Debug)]
pub struct JSErrorInfo {
    message: Option<String>,
    stack: Option<String>,
}

impl fmt::Display for JSErrorInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        match (&self.message, &self.stack) {
            (Some(msg), Some(stack)) => write!(f, "{}\n{}", msg, stack),
            (Some(msg), None) => write!(f, "{}", msg),
            (None, Some(stack)) => write!(f, "{}", stack),
            (None, None) => write!(f, "No error information!"),
        }
    }
}

impl std::error::Error for JSErrorInfo {}

pub trait JSExceptionHandler: JSContextKind {
    type Value: JSValueKind<Context = Self>;

    fn throw_syntax_error(&self, message: impl AsRef<str>) -> Self::Value;
    fn throw_type_error(&self, message: impl AsRef<str>) -> Self::Value;
    fn throw_reference_error(&self, message: impl AsRef<str>) -> Self::Value;
    fn throw_range_error(&self, message: impl AsRef<str>) -> Self::Value;
    fn throw_error(&self, message: impl AsRef<str>) -> Self::Value;
}

impl<C> JSContext<C>
where
    C: JSContextKind + JSExceptionHandler,
    C::Value: JSValueKind,
{
    pub fn throw_syntax_error(&self, message: impl AsRef<str>) -> JSValue<C::Value> {
        let raw = self.inner.throw_syntax_error(message);
        JSValue::new(self, raw)
    }

    pub fn throw_type_error(&self, message: impl AsRef<str>) -> JSValue<C::Value> {
        let raw = self.inner.throw_type_error(message);
        JSValue::new(self, raw)
    }

    pub fn throw_reference_error(&self, message: impl AsRef<str>) -> JSValue<C::Value> {
        let raw = self.inner.throw_reference_error(message);
        JSValue::new(self, raw)
    }

    pub fn throw_range_error(&self, message: impl AsRef<str>) -> JSValue<C::Value> {
        let raw = self.inner.throw_range_error(message);
        JSValue::new(self, raw)
    }

    pub fn throw_error(&self, message: impl AsRef<str>) -> JSValue<C::Value> {
        let raw = self.inner.throw_error(message);
        JSValue::new(self, raw)
    }
}
