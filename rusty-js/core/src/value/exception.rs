use crate::{FromJSValue, JSContext, JSContextImpl, JSObject, JSObjectOps, JSValue, JSValueImpl};
use std::fmt;
use std::ops::Deref;
use std::string::String;

pub struct JSException<'ctx, V: JSValueImpl>(JSObject<'ctx, V>);

impl<'ctx, V: JSObjectOps<'ctx>> Deref for JSException<'ctx, V> {
    type Target = JSObject<'ctx, V>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'ctx, V> JSException<'ctx, V>
where
    V: JSValueImpl,
{
    pub(crate) fn from_object(v: JSObject<'ctx, V>) -> Self {
        Self(v)
    }
}

impl<'ctx, V> JSException<'ctx, V>
where
    V: JSObjectOps<'ctx>,
    V::Context: JSExceptionHandler<Value = V>,
{
    pub fn from_message(ctx: &'ctx JSContext<V::Context>, message: &str) -> Self {
        let v = ctx.inner.new_error();
        let obj: JSObject<'ctx, V> = JSValue::new(ctx, v).into();
        obj.set("message", message);
        Self(obj)
    }
}

impl<'ctx, V> JSException<'ctx, V>
where
    V: JSObjectOps<'ctx>,
{
    pub fn into_error(self) -> JSErrorInfo {
        if self.is_error().is_some() {
            JSErrorInfo {
                message: self.message(),
                stack: self.stack(),
            }
        } else {
            JSErrorInfo {
                stack: None,
                message: Some(String::from_js_value(self.0.into_value()).unwrap()),
            }
        }
    }
}

impl<'ctx, V> JSException<'ctx, V>
where
    V: JSObjectOps<'ctx>,
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
            (None, None) => Ok(()),
        }
    }
}

impl std::error::Error for JSErrorInfo {}

pub trait JSExceptionHandler: JSContextImpl {
    type Value: JSValueImpl<Context = Self>;

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

impl<'ctx, V: JSObjectOps<'ctx>> fmt::Debug for JSException<'ctx, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Exception")
            .field("message", &self.message())
            .field("stack", &self.stack())
            .finish()
    }
}

impl<'ctx, V: JSObjectOps<'ctx>> fmt::Display for JSException<'ctx, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "Error:".fmt(f)?;
        if let Some(message) = &self.message() {
            ' '.fmt(f)?;
            message.fmt(f)?;
        }
        if let Some(stack) = &self.stack() {
            '\n'.fmt(f)?;
            stack.fmt(f)?;
        }
        Ok(())
    }
}
