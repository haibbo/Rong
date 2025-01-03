#[allow(unused_imports)]
pub use rusty_js::function::{ArgThis, Constructor, Optional, Rest, This, ThisMut};

pub use rusty_js::*;

pub fn run<F: FnOnce(&JSContext)>(f: F) {
    let rt = ActiveJSEngine::runtime();
    let ctx = ActiveJSEngine::context(&rt);
    f(&ctx);
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
