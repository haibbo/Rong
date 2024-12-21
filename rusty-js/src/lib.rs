pub use rusty_js_core::{
    FromJSValue, IntoJSValue, JSClass, JSContext as CoreJSContext, JSException as CoreJSException,
    JSFunc as CoreJSFunc, JSObject as CoreJSObject, JSRuntime as CoreJSRuntime,
    JSValue as CoreJSValue, RustFunc,
};

#[cfg(feature = "quickjs")]
mod engine {
    use rusty_js_quickjs::QJSRuntime;
    pub use rusty_js_quickjs::{QJSContext, QJSValue};
    pub type JSContext = super::CoreJSContext<QJSContext>;
    pub type JSRuntime = super::CoreJSRuntime<QJSRuntime>;
    pub type JSValue = super::CoreJSValue<QJSValue>;
    pub type JSObject = super::CoreJSObject<QJSValue>;
    pub type JSException = super::CoreJSException<QJSValue>;
    pub type JSFunc = super::CoreJSFunc<QJSValue>;
    pub type EJSValue = QJSValue;
    pub type EJSContext = QJSContext;
}

#[cfg(feature = "quickjs")]
pub use engine::*;
