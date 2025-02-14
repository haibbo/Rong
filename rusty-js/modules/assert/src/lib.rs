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
fn handle_assertion_error(message: Optional<JSValue>, default_message: &str) -> RustyJSError {
    if let Some(value) = message.0 {
        if let Ok(msg) = value.clone().try_into::<String>() {
            return RustyJSError::Error(msg);
        }

        if let Some(obj) = value.into_object() {
            // safe to unwrap, since it's Object
            let exception = JSException::from_object(obj).unwrap();
            return RustyJSError::Error(exception.into_error().to_string());
        }
    }
    RustyJSError::Error(default_message.to_string())
}

/// Asserts that two values are equal.
fn equal(left: JSValue, right: JSValue, message: Optional<JSValue>) -> JSResult<bool> {
    if left == right {
        Ok(true)
    } else {
        Err(handle_assertion_error(
            message,
            "AssertionError: It's not equal!",
        ))
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

    Err(handle_assertion_error(
        message,
        "AssertionError: The expression was evaluated to a falsy value",
    ))
}

/// Forces a test to fail with a custom message
fn fail(ctx: JSContext, message: Optional<JSValue>) -> JSValue {
    if let Some(msg) = message.0 {
        msg
    } else {
        ctx.throw_error("Failed")
    }
}

/// Asserts that a function does not throw an error
fn does_not_throw(ctx: JSContext, func: JSFunc, message: Optional<JSValue>) -> JSValue {
    // Call the function and check if it throws an error
    if func.call::<_, ()>((JSValue::undefined(&ctx),)).is_err() {
        if let Some(msg) = message.0 {
            return msg;
        }
    }
    // If no error was thrown, return undefined
    JSValue::undefined(&ctx)
}

pub fn init(ctx: &JSContext) -> JSResult<()> {
    let ok = ctx.register_function(ok)?.name("ok")?;
    let equal = ctx.register_function(equal)?.name("equal")?;
    let fail = ctx.register_function(fail)?.name("fail")?;
    let does_not_throw = ctx
        .register_function(does_not_throw)?
        .name("doesNotThrow")?;

    ok.set("ok", ok.clone())?
        .set("default", ok.clone())?
        .set("equal", equal)?
        .set("fail", fail)?
        .set("doesNotThrow", does_not_throw)?;
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
