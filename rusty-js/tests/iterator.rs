use rustyjs_test::*;

#[test]
fn iterator_sync() {
    run(|ctx: &JSContext| {
        ctx.global()
            .set("print", JSFunc::new(ctx, |msg: String| println!("{}", msg)))?;

        let data = vec![1, 2, 3, 4, 5];
        let iterator = JSFunc::new(ctx, move |ctx: JSContext| data.to_js_iter(&ctx));

        ctx.global().set("iterator", iterator)?;
        let result: i32 = ctx.eval(Source::from_bytes(
            r#"
            for (const n of iterator()) {
                 print(n.toString());
            }
            let sum=0;
            for (const n of iterator()) {
                 print(n.toString());
                 sum+=n;
            }
            sum
            "#,
        ))?;
        assert_eq!(result, 15);
        Ok(())
    });
}
