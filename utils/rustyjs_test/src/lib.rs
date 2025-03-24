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

pub struct UnitJSRunner<'a> {
    ctx: &'a JSContext,
}

impl<'a> UnitJSRunner<'a> {
    /// Load and execute the specified JavaScript test script, returning a UnitJSRunner instance
    pub async fn load_script(ctx: &'a JSContext, unit: &str) -> JSResult<Self> {
        let current_dir = std::env::current_dir().unwrap();

        // First, load the test runner
        let runner = current_dir.join("../../tests/unit/test-runner.js");
        let source = Source::from_path(ctx, runner).await?;
        ctx.eval_async::<()>(source).await?;

        // Then, load the test file
        let test = current_dir.join("../../tests/unit/").join(unit);
        let source = Source::from_path(ctx, test).await?;
        ctx.eval_async::<()>(source).await?;

        Ok(Self { ctx })
    }

    /// Run all tests and return true if all tests passed
    pub async fn run(&self) -> JSResult<bool> {
        // Execute the test and wait for completion
        let result: bool = self
            .ctx
            .eval_async(Source::from_bytes("runner.runTests()"))
            .await?;

        Ok(result)
    }
}
