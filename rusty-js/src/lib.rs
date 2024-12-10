pub use rusty_js_core::{
    FromJSValue, JSContext as CoreJSContext, JSException as CoreJSException,
    JSObject as CoreJSObject, JSRuntime as CoreJSRuntime, JSValue as CoreJSValue, JSValueTo,
};

#[cfg(feature = "quickjs")]
mod engine {
    use rusty_js_quickjs::{QJSContext, QJSRuntime, QJSValue};
    pub type JSContext = super::CoreJSContext<QJSContext>;
    pub type JSRuntime = super::CoreJSRuntime<QJSRuntime>;
    pub type JSValue<'ctx> = super::CoreJSValue<'ctx, QJSValue>;
    pub type JSObject<'ctx> = super::CoreJSObject<'ctx, QJSValue>;
    pub type JSException<'ctx> = super::CoreJSException<'ctx, QJSValue>;
}

#[cfg(feature = "quickjs")]
pub use engine::*;
