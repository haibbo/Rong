pub use rusty_js_core::{
    Exception as CoreJSException, JSContext as CoreJSContext, JSObject as CoreJSObject,
    JSRuntime as CoreJSRuntime, JSValue as CoreJSValue,
};

#[cfg(feature = "quickjs")]
mod engine {
    use rusty_js_quickjs::{QJSContext, QJSRuntime, QJSValue};
    pub type JSContext = super::CoreJSContext<QJSContext>;
    pub type JSRuntime = super::CoreJSRuntime<QJSRuntime>;
    pub type JSValue<'ctx> = super::CoreJSValue<'ctx, QJSValue>;
    pub type JSObject<'ctx> = super::CoreJSObject<'ctx, QJSValue>;
    pub type Exception<'ctx> = super::CoreJSException<'ctx, QJSValue>;
}

#[cfg(feature = "quickjs")]
pub use engine::*;
