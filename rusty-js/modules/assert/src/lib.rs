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
fn handle_assertion_error(
    ctx: &JSContext,
    message: Optional<JSValue>,
    default_message: &str,
) -> JSValue {
    message
        .0
        .map_or_else(|| ctx.throw_error(default_message), |value| value)
}

/// Asserts that two values are equal.
fn equal(ctx: JSContext, left: JSValue, right: JSValue, message: Optional<JSValue>) -> JSValue {
    if left == right {
        JSValue::from(&ctx, true)
    } else {
        handle_assertion_error(&ctx, message, "AssertionError: It's not equal!")
    }
}

/// Asserts that a value is truthy.
fn ok(ctx: JSContext, value: JSValue, message: Optional<JSValue>) -> JSValue {
    let undefined = JSValue::undefined(&ctx);
    match value.type_of() {
        JSValueType::Boolean => {
            if value.try_into::<bool>().unwrap_or(false) {
                return undefined;
            }
        }
        JSValueType::Number => {
            if value.try_into::<i32>().map(|b| b != 0).unwrap_or(false) {
                return undefined;
            }
        }
        JSValueType::String => {
            if value
                .try_into::<String>()
                .map(|s| !s.is_empty())
                .unwrap_or(false)
            {
                return undefined;
            }
        }
        JSValueType::Array
        | JSValueType::BigInt
        | JSValueType::Constructor
        | JSValueType::Exception
        | JSValueType::Function
        | JSValueType::Symbol
        | JSValueType::Object => {
            return undefined;
        }
        _ => {}
    }

    handle_assertion_error(
        &ctx,
        message,
        "AssertionError: The expression was evaluated to a falsy value",
    )
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
    let ok = JSFunc::new(ctx, ok)?.name("ok")?;
    let equal = JSFunc::new(ctx, equal)?.name("equal")?;
    let fail = JSFunc::new(ctx, fail)?.name("fail")?;
    let does_not_throw = JSFunc::new(ctx, does_not_throw)?.name("doesNotThrow")?;

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
            let passed = UnitJSRunner::load_script(&ctx, "assert.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            Ok(())
        });
    }
}
