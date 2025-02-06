pub use rusty_js::*;
// Re-export commonly used types for tests
pub use rusty_js::function::{Constructor, Optional, Rest, This, ThisMut};

// Helper function to run tests with JS context
#[allow(dead_code)]
pub fn run<F: FnOnce(&JSContext)>(f: F) {
    let rt = RustyJS::runtime();
    let ctx = RustyJS::context(&rt);
    f(&ctx);
}

// Helper function to run tests with both JS context and runtime
#[allow(dead_code)]
pub fn run2<F: FnOnce(&JSContext, &JSRuntime)>(f: F) {
    let rt = RustyJS::runtime();
    let ctx = RustyJS::context(&rt);
    f(&ctx, &rt);
}

#[macro_export]
macro_rules! assert_some {
    ($expr:expr) => {
        assert!($expr.is_some())
    };
}

#[macro_export]
macro_rules! assert_none {
    ($expr:expr) => {
        assert!($expr.is_none())
    };
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
