use rustyjs_test::*;

use std::string::String;
use tokio::time::Duration;

#[test]
fn test_eval() {
    run(|ctx| {
        let result: i32 = ctx.eval(Source::from_bytes(b"Math.sqrt(16)")).unwrap();
        assert_eq!(4, result);

        let result: String = ctx.eval(Source::from_bytes(b"'hi'")).unwrap(); // don't forget ''
        assert_eq!(String::from("hi"), result);

        let obj = ctx.global();
        assert_some!(obj.is_object());
    });
}

#[test]
fn test_bytecode() {
    run(|ctx| {
        let code = "(4 + 8) * 3";
        let source = match ctx.compile_to_bytecode(code) {
            Ok(source) => source,
            Err(RustyJSError::NotSupportByteCode) => return,
            Err(e) => panic!("Unexpected error: {:?}", e),
        };

        let result: i32 = ctx.eval(source).unwrap();
        assert_eq!(result, 36);
    });
}

#[test]
fn test_eval_async() {
    async_run!(|ctx: JSContext| async move {
        let set_timeout = ctx.register_function(|callback: JSFunc, delay: u32| {
            let future = async move {
                tokio::time::sleep(Duration::from_millis(delay as u64)).await;
                callback.call::<_, ()>(()).unwrap();
            };
            tokio::task::spawn_local(future);
        });
        ctx.global().set("setTimeout", set_timeout);

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
        let result: i32 = ctx.eval_async(source).await.unwrap();
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
            Err(RustyJSError::NotSupportByteCode) => return Ok(()),
            Err(e) => panic!("Unexpected error: {:?}", e),
        };
        // println!("source length is {}", source.len());

        let result: i32 = ctx.eval_async(source).await.unwrap();
        assert_eq!(result, 25);
        Ok(())
    })
}
