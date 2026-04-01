//! Command execution APIs attached to `globalThis.Rong`.

mod child_process;
mod io;
mod shell;
mod sync_process;

use rong::{IntoJSValue, JSArray, JSContext, JSObject, JSResult, JSValue};
use std::env;

fn create_env_object(ctx: &JSContext) -> JSResult<JSObject> {
    let env_obj = JSObject::new(ctx);
    for (key, value) in env::vars() {
        env_obj.set(key.as_str(), value)?;
    }
    Ok(env_obj)
}

fn create_string_array(
    ctx: &JSContext,
    values: impl IntoIterator<Item = String>,
) -> JSResult<JSValue> {
    let array = JSArray::new(ctx)?;
    for value in values {
        array.push(value)?;
    }
    Ok(array.into_js_value(ctx))
}

pub fn init(ctx: &JSContext) -> JSResult<()> {
    let rong = ctx.host_namespace();
    rong.set("env", create_env_object(ctx)?)?;
    rong.set("argv", create_string_array(ctx, env::args())?)?;
    rong.set("args", create_string_array(ctx, env::args().skip(2))?)?;

    rong_buffer::init(ctx)?;
    rong_encoding::init(ctx)?;
    rong_abort::init(ctx)?;
    rong_stream::init(ctx)?;
    io::init(ctx)?;
    child_process::init(ctx)?;
    sync_process::init(ctx)?;
    shell::init(ctx)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rong_test::*;

    fn run_unit_suite(unit: &str) {
        let unit = unit.to_string();
        async_run!(|ctx: JSContext| async move {
            rong_assert::init(&ctx)?;
            rong_console::init(&ctx)?;
            init(&ctx)?;

            let passed = UnitJSRunner::load_script(&ctx, &unit).await?.run().await?;
            assert!(passed);

            Ok(())
        });
    }

    fn run_stdio_suite(mode: &str) {
        let mode = mode.to_string();
        async_run!(|ctx: JSContext| async move {
            rong_assert::init(&ctx)?;
            rong_console::init(&ctx)?;
            let stdio = io::install_captured_stdio(&ctx, b"stdin-payload".to_vec());
            init(&ctx)?;
            ctx.global().set("__stdioMode", mode.clone())?;

            let passed = UnitJSRunner::load_script(&ctx, "stdio.js")
                .await?
                .run()
                .await?;
            assert!(passed);

            if mode == "text" {
                assert_eq!(
                    String::from_utf8(stdio.stdout_bytes()).unwrap(),
                    "hello-out"
                );
                assert_eq!(String::from_utf8(stdio.stderr_bytes()).unwrap(), "warn-err");
            }

            Ok(())
        });
    }

    #[test]
    fn test_command_namespace() {
        for unit in ["spawn.js", "shell.js"] {
            run_unit_suite(unit);
        }
        run_stdio_suite("text");
        run_stdio_suite("bytes");
    }
}
