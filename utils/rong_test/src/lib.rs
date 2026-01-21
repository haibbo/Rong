pub use rong::*;
// Re-export commonly used types for tests
pub use rong::function::{Constructor, Optional, Rest, This, ThisMut};

pub fn thrown_js_value(ctx: &JSContext, err: &RongJSError) -> JSResult<JSValue> {
    err.thrown_value(ctx)
        .ok_or_else(|| HostError::new(rong::error::E_INTERNAL, "Expected thrown JS value").into())
}

pub fn thrown_object(ctx: &JSContext, err: &RongJSError) -> JSResult<JSObject> {
    let thrown = thrown_js_value(ctx, err)?;
    thrown.into_object().ok_or_else(|| {
        HostError::new(
            rong::error::E_INTERNAL,
            "Expected thrown value to be an object",
        )
        .into()
    })
}

pub fn thrown_object_prop<T>(ctx: &JSContext, err: &RongJSError, key: &str) -> JSResult<T>
where
    T: FromJSValue<JSEngineValue>,
{
    thrown_object(ctx, err)?.get(key)
}

pub fn thrown_error_message(ctx: &JSContext, err: &RongJSError) -> JSResult<String> {
    thrown_object_prop(ctx, err, "message")
}

pub fn thrown_error_stack(ctx: &JSContext, err: &RongJSError) -> JSResult<String> {
    thrown_object_prop(ctx, err, "stack")
}

// Helper function to run tests with JS context
#[allow(dead_code)]
pub fn run<F: FnOnce(&JSContext) -> JSResult<()>>(f: F) {
    let rt = RongJS::runtime();
    let ctx = rt.context();
    f(&ctx).unwrap();
}

#[macro_export]
macro_rules! async_run {
    ($user_fn:expr) => {{
        let rong = Rong::<RongJS>::builder().build();

        let block_on_closure = |runtime: JSRuntime, _receiver| {
            let ctx = runtime.context();
            $user_fn(ctx)
        };

        rong.block_on::<_, _, ()>(block_on_closure).unwrap();
    }};
}

pub struct UnitJSRunner<'a> {
    ctx: &'a JSContext,
}

impl<'a> UnitJSRunner<'a> {
    /// Load and execute the specified JavaScript test script, returning a UnitJSRunner instance
    pub async fn load_script(ctx: &'a JSContext, unit: &str) -> JSResult<Self> {
        // Use CARGO_MANIFEST_DIR to find the test files relative to the crate root
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let base_path = manifest_dir.join("../../tests/unit");

        // Canonicalize the path to resolve any potential issues (like '..')
        let test_base_dir = match std::fs::canonicalize(&base_path) {
            Ok(path) => path,
            Err(e) => {
                // Provide more context in case of error
                return Err(HostError::new(
                    rong::error::E_INTERNAL,
                    format!(
                        "Failed to canonicalize test base path '{}': {}",
                        base_path.display(),
                        e
                    ),
                )
                .into());
            }
        };

        // First, load the test runner
        let runner_path = test_base_dir.join("test-runner.js");
        let source = Source::from_path(ctx, runner_path).await?;
        ctx.eval_async::<()>(source).await?;

        // Then, load the test file
        let test_path = test_base_dir.join(unit);
        let source = Source::from_path(ctx, test_path).await?;
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
