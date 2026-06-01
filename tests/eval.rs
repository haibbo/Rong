use rong_test::*;

use std::string::String;
use tokio::time::Duration;

fn compile_to_bytecode_or_skip(ctx: &JSContext, code: &str) -> JSResult<Option<Source>> {
    match ctx.compile_to_bytecode(code) {
        Ok(source) => Ok(Some(source)),
        Err(e)
            if e.is_not_support_bytecode()
                && std::env::var("RONG_JSC_REQUIRE_BYTECODE").as_deref() != Ok("1") =>
        {
            Ok(None)
        }
        Err(e) => panic!("Unexpected bytecode compile error: {:?}", e),
    }
}

#[test]
fn test_eval() {
    run(|ctx| {
        let result: i32 = ctx.eval(Source::from_bytes(b"Math.sqrt(16)"))?;
        assert_eq!(4, result);

        let result: String = ctx.eval(Source::from_bytes(b"'hi'"))?; // don't forget ''
        assert_eq!(String::from("hi"), result);

        let obj = ctx.global();
        assert!(obj.is_object());
        Ok(())
    });
}

#[test]
fn test_bytecode() {
    run(|ctx| {
        let code = "(4 + 8) * 3";
        let Some(source) = compile_to_bytecode_or_skip(ctx, code)? else {
            return Ok(());
        };

        let result: i32 = ctx.eval(source)?;
        assert_eq!(result, 36);
        Ok(())
    });
}

#[test]
fn test_compile_to_bytecode_does_not_execute() {
    run(|ctx| {
        let code = "globalThis.__rong_compile_side_effect = (globalThis.__rong_compile_side_effect || 0) + 1; 7";
        let Some(source) = compile_to_bytecode_or_skip(ctx, code)? else {
            return Ok(());
        };

        let side_effect_after_compile: i32 = ctx.eval(Source::from_bytes(
            "globalThis.__rong_compile_side_effect || 0",
        ))?;
        assert_eq!(side_effect_after_compile, 0);

        let result: i32 = ctx.eval(source)?;
        assert_eq!(result, 7);

        let side_effect_after_run: i32 = ctx.eval(Source::from_bytes(
            "globalThis.__rong_compile_side_effect || 0",
        ))?;
        assert_eq!(side_effect_after_run, 1);
        Ok(())
    });
}

#[test]
fn test_eval_async() {
    async_run!(|ctx: JSContext| async move {
        let set_timeout = JSFunc::new(&ctx, |callback: JSFunc, delay: u32| {
            let future = async move {
                tokio::time::sleep(Duration::from_millis(delay as u64)).await;
                callback.call::<_, ()>(None, ()).unwrap()
            };
            spawn_local(future);
            Ok(())
        })?;
        ctx.global().set("setTimeout", set_timeout)?;

        // Create Promise in JavaScript
        let js_code = r#"
            new Promise((resolve) => {
                setTimeout(() => {
                    resolve(42);
                }, 100);
            })
        "#;
        let source = Source::from_bytes(js_code);
        println!("source length is {}", source.len());
        let result: i32 = ctx.eval_async(source).await?;
        assert_eq!(result, 42);

        let js_code = r#"
            new Promise((resolve) => {
                setTimeout(() => {
                    resolve(10+15);
                }, 100);
            })
        "#;

        let Some(source) = compile_to_bytecode_or_skip(&ctx, js_code)? else {
            return Ok(());
        };
        // println!("source length is {}", source.len());

        let result: i32 = ctx.eval_async(source).await?;
        assert_eq!(result, 25);
        Ok(())
    })
}
