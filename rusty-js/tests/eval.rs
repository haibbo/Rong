mod helper;
use helper::*;

use std::string::String;

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
        let bytes = ctx.compile_to_bytecode(Source::from_bytes(code)).unwrap();
        println!("bytes.len is {}", bytes.len());

        let result: i32 = ctx.run_bytecode(&bytes).unwrap();
        assert_eq!(result, 36);
    });
}
