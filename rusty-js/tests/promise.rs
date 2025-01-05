mod helper;
use helper::*;

#[test]
fn test_promise_resolution_with_callback() {
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
