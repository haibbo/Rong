mod helper;
use helper::*;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

#[test]
fn test_rust_promise_with_callback() {
    run2(|ctx, rt| {
        let ctx_clone = ctx.clone();
        // Register a Rust function that returns a promise
        let timeout_fn = ctx.register_function(move |millis: i32| {
            let (promise, resolve, _reject) = ctx_clone.promise().unwrap();

            // Directly resolve the promise with the input value
            resolve.call::<_, ()>((millis,)).unwrap();
            promise
        });

        // Create a JS context to test the promise
        let js_code = r#"
                let result=10;
                rustTimeout(101).then((timeout)=>result=timeout);
                result
        "#;

        // Register the Rust function in JS context
        ctx.global().set("rustTimeout", timeout_fn);

        // Execute the JS code
        ctx.eval::<()>(Source::from_bytes(js_code.as_bytes()))
            .unwrap();

        // Run pending jobs until the promise resolves
        rt.run_pending_jobs();
        let result: i32 = ctx.eval(Source::from_bytes("result")).unwrap();

        assert_eq!(result, 101);
    });
}

#[test]
fn test_rust_promise_with_resolve() {
    run2(|ctx, rt| {
        let (promise, resolve, _reject) = ctx.promise().unwrap();

        // Use Rc<RefCell> for single-threaded shared mutability
        let result = std::rc::Rc::new(std::cell::RefCell::new(None));
        let result_clone = result.clone();

        let cb = ctx.register_function(move |value: String| {
            println!("Callback received value: {}", value);
            *result_clone.borrow_mut() = Some(value);
        });

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
    });
}

#[tokio::test(flavor = "current_thread")]
async fn test_rust_future_in_js() {
    run_local(|ctx, rt| async move {
        let ctx_for_future = ctx.clone();
        // Register a Rust async function that returns a promise
        let async_fn = ctx.register_function(move |delay: i32| {
            let ctx = ctx_for_future.clone();
            let future = async move {
                tokio::time::sleep(Duration::from_millis(delay as u64)).await;
                Ok(format!("completed after {}ms", delay))
            };
            Promise::from_future(&ctx, future).unwrap()
        });

        // Register the function in JS context
        ctx.global().set("rustAsync", async_fn);

        // Create JS code that uses the async function
        let js_code = r#"
            let result = 'pending';
            rustAsync(50)
                .then(msg => { result = msg; })
                .catch(err => { result = err; });
            result
        "#;

        // Execute the JS code
        ctx.eval::<()>(Source::from_bytes(js_code.as_bytes()))
            .unwrap();

        // Initial result should be 'pending'
        let initial: String = ctx.eval(Source::from_bytes("result")).unwrap();
        assert_eq!(initial, "pending");

        // Wait a bit longer than the sleep duration
        tokio::time::sleep(Duration::from_millis(60)).await;
        rt.run_pending_jobs();

        // Check the final result
        let current: String = ctx.eval(Source::from_bytes("result")).unwrap();
        assert_eq!(current, "completed after 50ms");
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_rust_future_error_in_js() {
    run_local(|ctx, rt| async move {
        let ctx_for_future = ctx.clone();
        // Register a Rust async function that returns a rejected promise
        let async_fn = ctx.register_function(move |_: i32| {
            let ctx = ctx_for_future.clone();
            let future = async {
                tokio::time::sleep(Duration::from_millis(50)).await;
                Err::<String, _>(RustyJSError::Error("async operation failed".to_string()))
            };
            Promise::from_future(&ctx, future).unwrap()
        });

        // Register the function in JS context
        ctx.global().set("rustAsyncError", async_fn);

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
        ctx.eval::<()>(Source::from_bytes(js_code)).unwrap();

        // Initial result should be 'pending'
        let initial: String = ctx.eval(Source::from_bytes("result")).unwrap();
        assert_eq!(initial, "pending");

        tokio::time::sleep(Duration::from_millis(60)).await;
        rt.run_pending_jobs();

        // Check the final result
        let result: String = ctx.eval(Source::from_bytes("result")).unwrap();
        assert_eq!(result, "rejected");

        // Verify the error message
        let error_message: String = ctx.eval(Source::from_bytes("errorMessage")).unwrap();
        assert_eq!(error_message, "async operation failed");
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_promise_into_future_resolve() {
    run_local(|ctx, rt| async move {
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

        let promise = ctx
            .eval::<Promise>(Source::from_bytes(js_code.as_bytes()))
            .unwrap();
        println!("Promise created");

        // Convert Promise to Future
        let mut future = promise.into_future();

        let result = loop {
            // Check if the future is ready
            let mut future_pin = Pin::new(&mut future);
            // Create a simple no-op waker
            struct NoopWaker;
            impl std::task::Wake for NoopWaker {
                fn wake(self: Arc<Self>) {}
            }

            let waker = std::task::Waker::from(Arc::new(NoopWaker));
            if let std::task::Poll::Ready(result) = future_pin
                .as_mut()
                .poll(&mut std::task::Context::from_waker(&waker))
            {
                break result;
            }

            // Run pending jobs
            rt.run_pending_jobs();

            // Sleep for a short time before checking again
            tokio::time::sleep(Duration::from_millis(1)).await;
        };

        let result: i32 = result.unwrap();
        assert_eq!(result, 42);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn test_promise_into_future_reject() {
    run_local(|ctx, rt| async move {
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
            new Promise((resolve, reject) => {
                setTimeout(() => {
                    reject(new Error("reject error"));
                }, 100);
            })
        "#;

        let promise = ctx
            .eval::<Promise>(Source::from_bytes(js_code.as_bytes()))
            .unwrap();
        println!("Promise created");

        // Convert Promise to Future
        let mut future = promise.into_future::<i32>();

        let result = loop {
            // Check if the future is ready
            let mut future_pin = Pin::new(&mut future);
            // Create a simple no-op waker
            struct NoopWaker;
            impl std::task::Wake for NoopWaker {
                fn wake(self: Arc<Self>) {}
            }

            let waker = std::task::Waker::from(Arc::new(NoopWaker));
            if let std::task::Poll::Ready(result) = future_pin
                .as_mut()
                .poll(&mut std::task::Context::from_waker(&waker))
            {
                break result;
            }

            // Run pending jobs
            rt.run_pending_jobs();

            // Sleep for a short time before checking again
            tokio::time::sleep(Duration::from_millis(1)).await;
        };

        let error = result.unwrap_err();
        assert!(error.to_string().contains("reject error"));
    })
    .await;
}
