//! # Assert Module
//!
//! This module provides assertion functions similar to Node.js's `assert` module.
//!
//! ### JavaScript Usage
//!
//! ```javascript
//! assert.ok(true);
//! assert.equal(1, 1);
//! ```

use rusty_js::{function::*, *};

/// Handles assertion errors with optional custom message
fn handle_assertion_error(message: Optional<JSValue>, default_message: &str) -> JSResult<bool> {
    if let Some(value) = message.0 {
        if let Ok(msg) = value.clone().try_into::<String>() {
            return Err(RustyJSError::Error(msg));
        }

        if let Some(obj) = value.into_object() {
            // safe to unwrap, since it's Object
            let exception = JSException::from_object(obj).unwrap();
            return Err(RustyJSError::Error(exception.into_error().to_string()));
        }
    }
    Err(RustyJSError::Error(default_message.to_string()))
}

/// Asserts that two values are equal.
fn equal(left: JSValue, right: JSValue, message: Optional<JSValue>) -> JSResult<bool> {
    if left == right {
        Ok(true)
    } else {
        handle_assertion_error(message, "AssertionError: It's not equal!")
    }
}

/// Asserts that a value is truthy.
fn ok(value: JSValue, message: Optional<JSValue>) -> JSResult<()> {
    match value.type_of() {
        JSValueType::Boolean => {
            if value.try_into()? {
                return Ok(());
            }
        }
        JSValueType::Number => {
            if value.try_into::<i32>()? != 0 {
                return Ok(());
            }
        }
        JSValueType::String => {
            if !value.try_into::<String>()?.is_empty() {
                return Ok(());
            }
        }
        JSValueType::Array
        | JSValueType::BigInt
        | JSValueType::Constructor
        | JSValueType::Exception
        | JSValueType::Function
        | JSValueType::Symbol
        | JSValueType::Object => {
            return Ok(());
        }
        _ => {}
    }

    handle_assertion_error(
        message,
        "AssertionError: The expression was evaluated to a falsy value",
    )?;
    Ok(())
}

pub fn init(ctx: &JSContext) -> JSResult<()> {
    let ok = ctx.register_function(ok)?.name("ok")?;
    let equal = ctx.register_function(equal)?;
    ok.set("ok", ok.clone())?
        .set("default", ok.clone())?
        .set("equal", equal)?;
    ctx.global().set("assert", ok)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustyjs_test::*;

    #[test]
    fn test_event() {
        async_run!(|ctx: JSContext| async move {
            ctx.global().set(
                "print",
                JSFunc::new(&ctx, |msg: String| println!("{}", msg)),
            )?;

            ctx.eval::<()>(Source::from_bytes(
                r#"
                    const console={
                        log: function(...args){
                            print(args.join(' '))
                        },
                        error: function(...args){
                            print(args.join(' '))
                        }
                    }
                "#,
            ))?;

            init(&ctx)?;

            let current_dir = std::env::current_dir().unwrap();

            let runner = current_dir.join("../../tests/unit/test-runner.js");
            let source = Source::from_path(runner).await.unwrap();
            ctx.eval_async::<()>(source).await?;

            let test = current_dir.join("../../tests/unit/assert.js");
            let source = Source::from_path(test).await.unwrap();
            ctx.eval_async::<()>(source).await?;

            let result: JSObject = ctx
                .eval_async(Source::from_bytes("runner.report()"))
                .await?;

            let failed: u32 = result.get("failed")?;
            let passed: u32 = result.get("passed")?;

            assert!(
                failed == 0,
                "Path tests passed: {}, failed: {}",
                failed,
                passed
            );
            Ok(())
        });
    }
}
