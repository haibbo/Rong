mod helper;
use helper::*;

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
        ctx.global_object().set("rustTimeout", timeout_fn);

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
