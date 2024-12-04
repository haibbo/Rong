pub use rusty_js::*;

pub fn run<F: FnOnce(&JSContext)>(f: F) {
    let rt = JSRuntime::new();
    let ctx = JSContext::new(&rt);
    f(&ctx);
}

#[macro_export]
macro_rules! assert_some {
    ($expr:expr) => {
        assert!($expr.is_some())
    };
}
