pub use rusty_js_core::{
    JSContext as CoreJSContext, JSRuntime as CoreJSRuntime, JSValue as CoreJSValue, JSValueFrom,
    JSValueInto,
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

#[cfg(test)]
pub(crate) fn test_with<F: FnOnce(&JSContext)>(f: F) {
    let rt = JSRuntime::new();
    let ctx = JSContext::new(&rt);
    f(&ctx);
}
mod test;
