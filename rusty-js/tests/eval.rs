mod helper;
use helper::*;

use std::string::String;

#[test]
fn test_eval() {
    run(|ctx| {
        let result: i32 = ctx.eval("Math.sqrt(16)").unwrap();
        assert_eq!(4, result);

        let result: String = ctx.eval("'hi'").unwrap(); // don't forget ''
        assert_eq!(String::from("hi"), result);
    });
}
