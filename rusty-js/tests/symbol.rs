use rustyjs_test::*;

#[test]
fn test_from_js() {
    run(|ctx| {
        let symbol: JSSymbol = ctx.eval(Source::from_bytes(b"Symbol('a b c')"))?;
        assert_eq!(symbol.descripiton()?, "a b c");
        Ok(())
    });
}

#[test]
fn test_symbol() {
    run(|ctx| {
        let symbol = JSSymbol::new(ctx, "a b c")?;
        assert!(symbol.is_symbol());
        assert_eq!(symbol.descripiton()?, "a b c");
        Ok(())
    });
}
