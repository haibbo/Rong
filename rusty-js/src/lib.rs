#[cfg(feature = "quickjs")]
mod engine {
    pub use rusty_js_quickjs::{JSCtxInner, JSRtInner, JSValueInner};
}
use engine::*;

mod context;
mod runtime;
mod value;

pub use context::JSCtx;
pub use runtime::JSRuntime;
pub use rusty_js_traits::{impl_js_values, FromHost, IntoHost};
pub use value::JSValue;

#[cfg(test)]
pub(crate) fn test_with<F: FnOnce(&JSCtx)>(f: F) {
    let rt = JSRuntime::new().unwrap();
    let ctx = JSCtx::new(&rt).unwrap();
    f(&ctx);
}
