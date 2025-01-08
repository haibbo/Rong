#[allow(unused_imports)]
pub use rusty_js::function::{ArgThis, Constructor, Optional, Rest, This, ThisMut};

pub use rusty_js::*;
use tokio::task::LocalSet;

// Helper function to run tests with JS context
#[allow(dead_code)]
pub fn run<F: FnOnce(&JSContext)>(f: F) {
    let rt = ActiveJSEngine::runtime();
    let ctx = ActiveJSEngine::context(&rt);
    f(&ctx);
}

// Helper function to run tests with both JS context and runtime
#[allow(dead_code)]
pub fn run2<F: FnOnce(&JSContext, &JSRuntime)>(f: F) {
    let rt = ActiveJSEngine::runtime();
    let ctx = ActiveJSEngine::context(&rt);
    f(&ctx, &rt);
}

// Helper function to run async tests with LocalSet
#[allow(dead_code)]
pub async fn run_local<F, Fut>(f: F)
where
    F: FnOnce(JSContext, JSRuntime) -> Fut + 'static,
    Fut: std::future::Future<Output = ()> + 'static,
{
    let local = LocalSet::new();
    let rt = ActiveJSEngine::runtime();
    let ctx = ActiveJSEngine::context(&rt);

    local.run_until(async move { f(ctx, rt).await }).await;
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
