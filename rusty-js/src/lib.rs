pub use rusty_js_core::{
    JSContext as CoreJSContext, JSRuntime as CoreJSRuntime, JSValue as CoreJSValue,
};

#[cfg(feature = "quickjs")]
mod engine {
    use rusty_js_quickjs::{QJSContext, QJSRuntime, QJSValue};
    pub type JSContext = super::CoreJSContext<QJSContext>;
    pub type JSRuntime = super::CoreJSRuntime<QJSRuntime>;
    pub type JSValue<'ctx> = super::CoreJSValue<'ctx, QJSValue>;
}

#[cfg(feature = "quickjs")]
pub use engine::*;
