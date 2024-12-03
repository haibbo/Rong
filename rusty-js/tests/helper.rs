pub use rusty_js::*;

pub fn run<F: FnOnce(&JSContext)>(f: F) {
    let rt = JSRuntime::new();
    let ctx = JSContext::new(&rt);
    f(&ctx);
}
