mod helper;
use helper::*;

#[test]
fn function_with_optional() {
    run(|ctx| {
        let func = ctx
            .register_function(|a: i32, b: Optional<i32>| match *b {
                Some(val) => a + val,
                None => a,
            })
            .name("add_optional");
        ctx.global_object().set("add_optional", func);

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
    });
}

#[test]
fn function_with_rest() {
    run(|ctx| {
        let func = ctx
            .register_function(|init: i32, rest: Rest<i32>| {
                let sum: i32 = rest.iter().sum();
                init + sum
            })
            .name("add");
        ctx.global_object().set("add_rest", func);

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
    });
}

#[test]
fn function_with_optional_and_rest() {
    run(|ctx| {
        let func = ctx
            .register_function(|a: i32, b: Optional<i32>, rest: Rest<i32>| {
                let base = match *b {
                    Some(val) => a + val,
                    None => a,
                };
                let sum: i32 = rest.iter().sum();
                base + sum
            })
            .name("complex_add");
        ctx.global_object().set("complex_add", func);

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
    });
}

#[test]
fn test_jsfunc_call() {
    run(|ctx| {
        // Test 1: Rust-created JS function
        let rust_func = JSFunc::new(ctx, |a: i32, b: i32| a + b);
        let result: i32 = rust_func.call((2, 3)).unwrap();
        assert_eq!(result, 5);

        // Test 2: JavaScript-created function
        let js_func: JSFunc = ctx
            .eval(Source::from_bytes(b"(function(a, b) { return a * b; })"))
            .unwrap();
        let result: i32 = js_func.call((4, 5)).unwrap();
        assert_eq!(result, 20);

        // Test 3: error. Rust clousre set the lenght of function.
        let result: Result<i32, _> = rust_func.call(());
        assert!(result.is_err());
    });
}

#[test]
fn test_jsfunc_call_macro() {
    run(|ctx| {
        // Test 1: 2 arguments
        let rust_func = JSFunc::new(ctx, |a: i32, b: i32| a + b);
        let result: i32 = call!(rust_func, 2, 3).unwrap();
        assert_eq!(result, 5);

        // Test 2: 0 argument
        let rust_func = JSFunc::new(ctx, || 8);
        let result: i32 = call!(rust_func).unwrap();
        assert_eq!(result, 8);
    });
}

#[test]
fn test_jsfunc_as_argument() {
    run(|ctx| {
        // Register a function that takes a JS function as argument
        let func = ctx
            .register_function(|callback: JSFunc| {
                // Call the JS function with some arguments
                let result: i32 = callback.call((2, 3)).unwrap();
                result * 2
            })
            .name("call_and_double");

        ctx.global_object().set("call_and_double", func);

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
    });
}
