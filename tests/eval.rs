use rong_test::*;

use std::string::String;
use tokio::time::Duration;

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
        let source = match ctx.compile_to_bytecode(code) {
            Ok(source) => source,
            Err(RongJSError::NotSupportByteCode) => return Ok(()),
            Err(e) => panic!("Unexpected error: {:?}", e),
        };

        let result: i32 = ctx.eval(source)?;
        assert_eq!(result, 36);
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
            spawn(future);
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

        let source = match ctx.compile_to_bytecode(js_code) {
            Ok(source) => source,
            Err(RongJSError::NotSupportByteCode) => return Ok(()),
            Err(e) => panic!("Unexpected error: {:?}", e),
        };
        // println!("source length is {}", source.len());

        let result: i32 = ctx.eval_async(source).await?;
        assert_eq!(result, 25);
        Ok(())
    })
}
