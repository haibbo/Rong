use rong_test::*;
use tokio::time::{Duration, sleep};

#[test]
fn function_with_optional() {
    run(|ctx| {
        let func = JSFunc::new(ctx, |a: i32, b: Optional<i32>| match *b {
            Some(val) => a + val,
            None => a,
        })?
        .name("add_optional")?;
        ctx.global().set("add_optional", func)?;

        assert_eq!(
            ctx.eval::<i32>(Source::from_bytes(b"add_optional(7)"))
                .unwrap(),
            7
        );
        assert_eq!(
            ctx.eval::<i32>(Source::from_bytes(b"add_optional(7, 3)"))
                .unwrap(),
            10
        );
        assert_eq!(
            ctx.eval::<i32>(Source::from_bytes(b"add_optional.length"))
                .unwrap(),
            1
        );
        Ok(())
    });
}

#[test]
fn function_with_rest() {
    run(|ctx| {
        let func = JSFunc::new(ctx, |init: i32, rest: Rest<i32>| {
            let sum: i32 = rest.iter().sum();
            init + sum
        })?
        .name("add")?;
        ctx.global().set("add_rest", func)?;

        assert_eq!(
            ctx.eval::<i32>(Source::from_bytes(b"add_rest(1)")).unwrap(),
            1
        );
        assert_eq!(
            ctx.eval::<i32>(Source::from_bytes(b"add_rest(1, 2)"))
                .unwrap(),
            3
        );
        assert_eq!(
            ctx.eval::<i32>(Source::from_bytes(b"add_rest(1, 2, 3, 4)"))
                .unwrap(),
            10
        );
        assert_eq!(
            ctx.eval::<i32>(Source::from_bytes(b"add_rest.length"))
                .unwrap(),
            1
        );
        Ok(())
    });
}

#[test]
fn function_with_optional_and_rest() {
    run(|ctx| {
        let func = JSFunc::new(ctx, |a: i32, b: Optional<i32>, rest: Rest<i32>| {
            let base = match *b {
                Some(val) => a + val,
                None => a,
            };
            let sum: i32 = rest.iter().sum();
            base + sum
        })?
        .name("complex_add")?;
        ctx.global().set("complex_add", func)?;

        assert_eq!(
            ctx.eval::<i32>(Source::from_bytes(b"complex_add(1)"))
                .unwrap(),
            1
        );
        assert_eq!(
            ctx.eval::<i32>(Source::from_bytes(b"complex_add(1, 2)"))
                .unwrap(),
            3
        );
        assert_eq!(
            ctx.eval::<i32>(Source::from_bytes(b"complex_add(1, 2, 3)"))
                .unwrap(),
            6
        );
        assert_eq!(
            ctx.eval::<i32>(Source::from_bytes(b"complex_add(1, 2, 3, 4)"))
                .unwrap(),
            10
        );
        assert_eq!(
            ctx.eval::<i32>(Source::from_bytes(b"complex_add.length"))
                .unwrap(),
            1
        );
        Ok(())
    });
}

#[test]
fn test_jsfunc_call() {
    run(|ctx| {
        // Test 1: Rust-created JS function
        let rust_func = JSFunc::new(ctx, |a: i32, b: i32| a + b)?;
        let result: i32 = rust_func.call(None, (2, 3)).unwrap();
        assert_eq!(result, 5);

        // Test 2: JavaScript-created function
        let js_func: JSFunc = ctx
            .eval(Source::from_bytes(b"(function(a, b) { return a * b; })"))
            .unwrap();
        let result: i32 = js_func.call(None, (4, 5)).unwrap();
        assert_eq!(result, 20);

        // Test 3: error. Rust clousre set the lenght of function.
        let result: Result<i32, _> = rust_func.call(None, ());
        assert!(result.is_err());
        Ok(())
    });
}

#[test]
fn test_jsfunc_call_macro() {
    run(|ctx| {
        // Test 1: 2 arguments
        let rust_func = JSFunc::new(ctx, |a: i32, b: i32| a + b)?;
        let result: i32 = rust_func.call(None, (2, 3)).unwrap();
        assert_eq!(result, 5);

        // Test 2: 0 argument
        let rust_func = JSFunc::new(ctx, || 8)?;
        let result: i32 = rust_func.call(None, ()).unwrap();
        assert_eq!(result, 8);
        Ok(())
    });
}

#[test]
fn test_jsfunc_as_argument() {
    run(|ctx| {
        // Register a function that takes a JS function as argument
        let func = JSFunc::new(ctx, |callback: JSFunc| {
            // Call the JS function with some arguments
            let result: i32 = callback.call(None, (2, 3)).unwrap();
            result * 2
        })?
        .name("call_and_double")?;

        ctx.global().set("call_and_double", func)?;

        // Test with a simple addition function
        let result: i32 = ctx
            .eval(Source::from_bytes(
                b"call_and_double(function(a, b) { return a + b; })",
            ))
            .unwrap();
        assert_eq!(result, 10); // (2 + 3) * 2

        // Test with a multiplication function
        let result: i32 = ctx
            .eval(Source::from_bytes(
                b"call_and_double(function(a, b) { return a * b; })",
            ))
            .unwrap();
        assert_eq!(result, 12); // (2 * 3) * 2
        Ok(())
    });
}

#[test]
fn test_async_rust_fn_resolve() {
    async_run!(|ctx: JSContext| async move {
        let async_func = JSFunc::new(&ctx, |a: i32, b: i32| async move {
            sleep(Duration::from_millis(100)).await;
            a + b
        })?;
        ctx.global().set("add", async_func)?;

        let result: i32 = ctx
            .eval::<Promise>(Source::from_bytes(
                b"add(2,6).then(result=>{return result;})",
            ))?
            .into_future()
            .await?;

        assert_eq!(result, 8);
        Ok(())
    });
}

#[test]
fn test_async_rust_fn_reject() {
    async_run!(|ctx: JSContext| async move {
        let async_func = JSFunc::new(&ctx, |_a: i32, _b: i32| async move {
            sleep(Duration::from_millis(100)).await;
            RongJSError::Error("Failed to perform add".to_string())
        })?; // PromiseResolver help call reject to propagate error to JS catch
        ctx.global().set("add", async_func)?;

        // catch trigger rust resolver callback
        let result = ctx
            .eval::<Promise>(Source::from_bytes(
                br#"add(2,6)
                .then((resolve) => {return resolve;})
                .catch(err =>{ return new Error(err+"!");})
                "#,
            ))?
            .into_future::<i32>()
            .await;

        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to perform add!")
        );
        Ok(())
    });
}

#[test]
fn test_new_once() {
    run(|ctx| {
        // Create a function that can only be called once
        let func = JSFunc::new_once(ctx, |x: i32| x + 1)?;
        ctx.global().set("once", func)?;

        // catch trigger rust resolver callback
        let result = ctx.eval::<i32>(Source::from_bytes("once(2); once(3)"));

        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("OnceFn had been called")
        );

        Ok(())
    });
}

#[test]
fn test_new_once_async() {
    async_run!(|ctx: JSContext| async move {
        let set_timeout = JSFunc::new_once(&ctx, |callback: JSFunc, delay: u32| async move {
            tokio::time::sleep(Duration::from_millis(delay as u64)).await;
            callback.call::<_, ()>(None, ()).unwrap();
        })?;
        ctx.global().set("setTimeout", set_timeout)?;

        // Create Promise in JavaScript
        let js_code = r#"
            new Promise((resolve) => {
                setTimeout(() => {
                    resolve(42);
                }, 100);
                setTimeout(() => {
                   resolve(42);
                }, 100);

            })
        "#;

        let promise = ctx
            .eval::<Promise>(Source::from_bytes(js_code.as_bytes()))
            .unwrap();

        let result: JSResult<i32> = promise.into_future().await;
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("OnceFn had been called")
        );

        tokio::time::sleep(Duration::from_millis(110)).await;
        Ok(())
    })
}

#[test]
fn test_call_async_sync_return() {
    async_run!(|ctx: JSContext| async move {
        // Test regular synchronous function with call_async
        let js_func: JSFunc = ctx
            .eval(Source::from_bytes(b"(function(a, b) { return a + b; })"))
            .unwrap();

        let result: i32 = js_func.call_async(None, (2, 3)).await?;
        assert_eq!(result, 5);

        Ok(())
    });
}

#[test]
fn test_call_async_promise_return() {
    async_run!(|ctx: JSContext| async move {
        // Create a sleep function using Rust
        let sleep_fn = JSFunc::new(&ctx, |ms: u32| async move {
            sleep(Duration::from_millis(ms as u64)).await;
        })?;
        ctx.global().set("sleep", sleep_fn)?;

        // Test async function that returns a Promise
        let js_func: JSFunc = ctx
            .eval(Source::from_bytes(
                b"(async function(a, b) {
                    await sleep(100);
                    return a * b;
                })",
            ))
            .unwrap();

        let result: i32 = js_func.call_async(None, (4, 5)).await?;
        assert_eq!(result, 20);

        Ok(())
    });
}

#[test]
fn test_call_async_with_this() {
    async_run!(|ctx: JSContext| async move {
        // Create a sleep function using Rust
        let sleep_fn = JSFunc::new(&ctx, |ms: u32| async move {
            sleep(Duration::from_millis(ms as u64)).await;
        })?;
        ctx.global().set("sleep", sleep_fn)?;

        // Create an object with a method that uses 'this'
        let obj: JSObject = ctx
            .eval(Source::from_bytes(
                b"({
                    value: 10,
                    asyncMethod: async function(x) {
                        await sleep(50);
                        return this.value + x;
                    }
                })",
            ))
            .unwrap();

        let method: JSFunc = obj.get("asyncMethod")?;

        // Call with 'this' context
        let result: i32 = method.call_async(Some(obj.clone()), (5,)).await?;
        assert_eq!(result, 15); // 10 + 5

        Ok(())
    });
}

#[test]
fn test_call_async_error_handling() {
    async_run!(|ctx: JSContext| async move {
        // Create a sleep function using Rust
        let sleep_fn = JSFunc::new(&ctx, |ms: u32| async move {
            sleep(Duration::from_millis(ms as u64)).await;
        })?;
        ctx.global().set("sleep", sleep_fn)?;

        // Function that throws an error after a delay
        let js_func: JSFunc = ctx
            .eval(Source::from_bytes(
                b"(async function() {
                    await sleep(50);
                    throw new Error('test error');
                })",
            ))
            .unwrap();

        let result: Result<i32, _> = js_func.call_async(None, ()).await;
        assert!(result.is_err());

        Ok(())
    });
}
