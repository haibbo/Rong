mod helper;
use helper::*;
use rusty_js_core::function::{Optional, Rest};

#[derive(Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

impl IntoJSValue<JSEngineValue> for Point {
    fn into_js_value(self, context: &JSEngineContext) -> JSEngineValue {
        Class::get::<Point>(context)
            .map(|class| class.instance(self))
            .unwrap_or_else(|| JSEngineValue::from((context, ())))
    }
}

impl FromJSValue<JSEngineValue> for Point {
    fn from_js_value(ctx: &JSEngineContext, value: JSEngineValue) -> Result<Self, String> {
        let obj = JSObject::from_js_value(ctx, value)?;
        let point = obj
            .borrow::<Point>()
            .ok_or_else(|| "Failed to borrow Point data".to_string())?;

        Ok(*point)
    }
}

impl JSClass<JSEngineValue> for Point {
    const NAME: &'static str = "Point";

    fn data_constructor() -> RustFunc {
        RustFunc::new(|x, y| Point { x, y })
    }
}

#[test]
fn constructor() {
    run(|ctx| {
        ctx.register_class::<Point>();
        let point = ctx.eval::<Point>("let point=new Point(2,3);point").unwrap();
        assert_eq!(point.x, 2);
        assert_eq!(point.y, 3);

        assert_eq!(
            ctx.eval::<String>("Point.constructor.name").unwrap(),
            "Function"
        );
        assert_eq!(ctx.eval::<String>("typeof Point").unwrap(), "function");
        assert!(ctx.eval::<bool>("point instanceof Point").unwrap());
    });
}

#[test]
fn function() {
    run(|ctx| {
        let func = ctx
            .register_function(|a: i32, b: i32, c: i32| a + b + c)
            .name("add");
        ctx.global_object().set("add", func);

        assert!(ctx.eval::<JSFunc>("add").is_ok());
        assert_eq!(ctx.eval::<i32>("add(7, 9,1)").unwrap(), 17);
        assert_eq!(ctx.eval::<i32>("add.length").unwrap(), 3);
        assert_eq!(ctx.eval::<String>("add.name").unwrap(), "add");
    });
}

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

        assert_eq!(ctx.eval::<i32>("add_optional(7)").unwrap(), 7);
        assert_eq!(ctx.eval::<i32>("add_optional(7, 3)").unwrap(), 10);
        assert_eq!(ctx.eval::<i32>("add_optional.length").unwrap(), 1);
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

        assert_eq!(ctx.eval::<i32>("add_rest(1)").unwrap(), 1);
        assert_eq!(ctx.eval::<i32>("add_rest(1, 2)").unwrap(), 3);
        assert_eq!(ctx.eval::<i32>("add_rest(1, 2, 3, 4)").unwrap(), 10);
        assert_eq!(ctx.eval::<i32>("add_rest.length").unwrap(), 1);
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

        assert_eq!(ctx.eval::<i32>("complex_add(1)").unwrap(), 1);
        assert_eq!(ctx.eval::<i32>("complex_add(1, 2)").unwrap(), 3);
        assert_eq!(ctx.eval::<i32>("complex_add(1, 2, 3)").unwrap(), 6);
        assert_eq!(ctx.eval::<i32>("complex_add(1, 2, 3, 4)").unwrap(), 10);
        assert_eq!(ctx.eval::<i32>("complex_add.length").unwrap(), 1);
    });
}
