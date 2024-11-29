use crate::{JSCodeRunner, JSValue, JSValueInto, JSValueKind};
use std::fmt;

/// extract exception and error details from JSValue
/// this trait can only be implemented by JSValue
pub trait JSValueError: JSValueKind
where
    Self: Sized,
    Self: JSValueInto<String>,
{
    fn get_exception_message(value: &JSValue<Self>) -> Option<String> {
        value.clone().into_rust()
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
    V::Context: JSCodeRunner<Value = V>,
{
    pub fn into_error(self) -> JSErrorInfo {
        let exception = self.as_ctx().get_last_exception();
        JSErrorInfo {
            message: V::get_exception_message(&exception),
            stack: V::get_exception_stack(&exception),
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
