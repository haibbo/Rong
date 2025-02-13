pub use rusty_js::*;
// Re-export commonly used types for tests
pub use rusty_js::function::{Constructor, Optional, Rest, This, ThisMut};

// Helper function to run tests with JS context
#[allow(dead_code)]
pub fn run<F: FnOnce(&JSContext) -> JSResult<()>>(f: F) {
    let rt = RustyJS::runtime();
    let ctx = RustyJS::context(&rt);
    f(&ctx).unwrap();
}

// Helper function to run tests with both JS context and runtime
#[allow(dead_code)]
pub fn run2<F: FnOnce(&JSContext, &JSRuntime) -> JSResult<()>>(f: F) {
    let rt = RustyJS::runtime();
    let ctx = RustyJS::context(&rt);
    f(&ctx, &rt).unwrap();
}

#[macro_export]
macro_rules! async_run {
    ($block:expr) => {{
        let rt = RustyJS::runtime();
        let ctx = RustyJS::context(&rt);
        let future = async move { $block(ctx).await };
        rt.block_on(future).unwrap()
    }};
}
