use rustyjs_test::*;
use std::time::Duration;

#[test]
fn test_rust_promise_with_callback() {
    run2(|ctx, rt| {
        let ctx_clone = ctx.clone();
        // Register a Rust function that returns a promise
        let timeout_fn = JSFunc::new(ctx, move |millis: i32| {
            let (promise, resolve, _reject) = ctx_clone.promise().unwrap();

            // Directly resolve the promise with the input value
            resolve.call::<_, ()>((millis,)).unwrap();
            promise
        })?;

        // Create a JS context to test the promise
        let js_code = r#"
                let result=10;
                rustTimeout(101).then((timeout)=>result=timeout);
                result
        "#;

        // Register the Rust function in JS context
        ctx.global().set("rustTimeout", timeout_fn)?;

        // Execute the JS code
        ctx.eval::<()>(Source::from_bytes(js_code.as_bytes()))
            .unwrap();

        // Run pending jobs until the promise resolves
        rt.run_pending_jobs();
        let result: i32 = ctx.eval(Source::from_bytes("result")).unwrap();

        assert_eq!(result, 101);
        Ok(())
    });
}

#[test]
fn test_rust_promise_with_resolve() {
    run2(|ctx, rt| {
        let (promise, resolve, _reject) = ctx.promise().unwrap();

        // Use Rc<RefCell> for single-threaded shared mutability
        let result = std::rc::Rc::new(std::cell::RefCell::new(None));
        let result_clone = result.clone();

        let cb = JSFunc::new(ctx, move |value: String| {
            println!("Callback received value: {}", value);
            *result_clone.borrow_mut() = Some(value);
        })?;

        let then = promise.then();
        then.call_with_this::<_, ()>(promise.into_object(), (cb,))
            .unwrap();

        // Resolve the promise
        resolve.call::<_, ()>(("success!",)).unwrap();

        // Run pending jobs to trigger the callback
        rt.run_pending_jobs();

        // Now assert the result after jobs have run
        let final_result = result.borrow().clone().expect("Callback was not called");
        assert_eq!(final_result, "success!");
        Ok(())
    });
}

#[test]
fn test_rust_future_in_js() {
    async_run!(|ctx: JSContext| async move {
        let ctx2 = ctx.clone();
        // Register a Rust async function that returns a promise
        let async_fn = JSFunc::new(&ctx, move |delay: i32| {
            let future = async move {
                tokio::time::sleep(Duration::from_millis(delay as u64)).await;
                format!("completed after {}ms", delay)
            };
            Promise::from_future(&ctx2, future).unwrap()
        })?;

        // Register the function in JS context
        ctx.global().set("rustAsync", async_fn)?;

        // Create JS code that uses the async function
        let js_code = r#"
            let result = 'pending';
            rustAsync(50)
                .then(msg => { result = msg; })
                .catch(err => { result = err; });
            result
        "#;

        // Execute the JS code
        ctx.eval::<()>(Source::from_bytes(js_code.as_bytes()))?;

        // Initial result should be 'pending'
        let initial: String = ctx.eval(Source::from_bytes("result"))?;
        assert_eq!(initial, "pending");

        // wait rustAsync finished
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Check the final result
        let current: String = ctx.eval(Source::from_bytes("result"))?;
        assert_eq!(current, "completed after 50ms");
        Ok(())
    })
}

#[test]
fn test_rust_future_error_in_js() {
    async_run!(|ctx: JSContext| async move {
        let ctx2 = ctx.clone();
        // Register a Rust async function that returns a rejected promise
        let async_fn = JSFunc::new(&ctx, move |_: i32| {
            let future = async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                RustyJSError::Error("async operation failed".to_string())
            };
            Promise::from_future(&ctx2, future).unwrap()
        })?;

        // Register the function in JS context
        ctx.global().set("rustAsyncError", async_fn)?;

        // Create JS code that uses the async function with more debug info
        let js_code = br#"
            let result = 'pending';
            let errorMessage = '';
            rustAsyncError(0)
                .then((msg) => {
                    result = 'resolved';
                })
                .catch((err) => {
                    result = 'rejected';
                    errorMessage = err.message;
                });
            result
        "#;

        // Execute the JS code
        ctx.eval::<()>(Source::from_bytes(js_code))?;

        // Initial result should be 'pending'
        let initial: String = ctx.eval(Source::from_bytes("result"))?;
        assert_eq!(initial, "pending");

        // wait rustAsyncError finished
        tokio::time::sleep(Duration::from_millis(60)).await;

        // Check the final result
        let result: String = ctx.eval(Source::from_bytes("result"))?;
        assert_eq!(result, "rejected");

        // Verify the error message
        let error_message: String = ctx.eval(Source::from_bytes("errorMessage"))?;
        assert_eq!(error_message, "async operation failed");
        Ok(())
    })
}

#[test]
fn test_promise_into_future_resolve() {
    async_run!(|ctx: JSContext| async move {
        let set_timeout = JSFunc::new(&ctx, |callback: JSFunc, delay: u32| {
            let future = async move {
                tokio::time::sleep(Duration::from_millis(delay as u64)).await;
                callback.call::<_, ()>(()).unwrap();
            };
            tokio::task::spawn_local(future);
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

        let promise = ctx
            .eval::<Promise>(Source::from_bytes(js_code.as_bytes()))
            .unwrap();

        let result: i32 = promise.into_future().await.unwrap();
        assert_eq!(result, 42);
        Ok(())
    })
}

#[test]
fn test_promise_into_future_reject_error() {
    async_run!(|ctx: JSContext| async move {
        let set_timeout = JSFunc::new(&ctx, |callback: JSFunc, delay: u32| {
            let future = async move {
                tokio::time::sleep(Duration::from_millis(delay as u64)).await;
                callback.call::<_, ()>(()).unwrap();
            };
            tokio::task::spawn_local(future);
        })?;
        ctx.global().set("setTimeout", set_timeout)?;

        let js_code = r#"
            new Promise((resolve, reject) => {
                setTimeout(() => {
                    reject(new Error("reject error"));
                }, 100);
            })
        "#;

        let promise = ctx.eval::<Promise>(Source::from_bytes(js_code.as_bytes()))?;

        let error = promise.into_future::<i32>().await.unwrap_err();
        assert!(error.to_string().contains("reject error"));
        Ok(())
    })
}

#[test]
fn test_promise_into_future_reject_exception() {
    async_run!(|ctx: JSContext| async move {
        let set_timeout = JSFunc::new(&ctx, |callback: JSFunc, delay: u32| async move {
            tokio::time::sleep(Duration::from_millis(delay as u64)).await;
            let _ = callback.call::<_, ()>(());
        })?;

        ctx.global().set("setTimeout", set_timeout)?;

        let js_code = r#"
            new Promise((resolve, reject) => {
                setTimeout(() => {
                    try {
                        throw new Error("timeout failure");
                    } catch (err) {
                        reject(err);
                    }
                }, 100);
            })
        "#;

        let promise = ctx.eval::<Promise>(Source::from_bytes(js_code.as_bytes()))?;

        let error = promise.into_future::<i32>().await.unwrap_err();
        assert!(error.to_string().contains("timeout failure"));
        Ok(())
    })
}

#[test]
fn test_rust_promise_with_mut_state() {
    run2(|ctx, rt| {
        let ctx_clone = ctx.clone();
        let mut counter = 0;

        // Register a function that captures mutable state
        let counter_fn = JSFunc::new(ctx, move || {
            let (promise, resolve, _) = ctx_clone.promise().unwrap();
            counter += 1;
            resolve.call::<_, ()>((counter,)).unwrap();
            promise
        })?;

        ctx.global().set("getCounter", counter_fn)?;

        // Call the function multiple times and store results
        let js_code = r#"
            let result1, result2;
            getCounter()
                .then(val => { result1 = val; });
            getCounter()
                .then(val => { result2 = val; });
        "#;

        ctx.eval::<()>(Source::from_bytes(js_code)).unwrap();
        rt.run_pending_jobs();

        // Check individual results
        let result1: i32 = ctx.eval(Source::from_bytes("result1")).unwrap();
        let result2: i32 = ctx.eval(Source::from_bytes("result2")).unwrap();
        assert_eq!(result1, 1);
        assert_eq!(result2, 2);
        Ok(())
    });
}

#[test]
fn test_rust_async_with_mut_state() {
    async_run!(|ctx: JSContext| async move {
        let ctx2 = ctx.clone();
        let mut counter = 0;

        let async_fn = JSFunc::new(&ctx, move || {
            counter += 1;
            let count = counter;

            let future = async move {
                tokio::time::sleep(Duration::from_millis(50)).await;
                format!("Counter: {}", count)
            };
            Promise::from_future(&ctx2, future).unwrap()
        })?;

        ctx.global().set("asyncCounter", async_fn)?;

        let js_code = r#"
            let result1, result2;
            asyncCounter().then(val => { result1 = val; });
            asyncCounter().then(val => { result2 = val; });
        "#;

        ctx.eval::<()>(Source::from_bytes(js_code))?;

        // Wait for promises to resolve
        tokio::time::sleep(Duration::from_millis(60)).await;

        let result1: String = ctx.eval(Source::from_bytes("result1"))?;
        let result2: String = ctx.eval(Source::from_bytes("result2"))?;
        assert_eq!(result1, "Counter: 1");
        assert_eq!(result2, "Counter: 2");
        Ok(())
    })
}
