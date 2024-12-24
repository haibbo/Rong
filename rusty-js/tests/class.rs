mod helper;
use helper::*;

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

        assert_eq!(ctx.eval::<i32>("add(7, 9,1)").unwrap(), 17);
        assert_eq!(ctx.eval::<i32>("add.length").unwrap(), 3);
        assert_eq!(ctx.eval::<String>("add.name").unwrap(), "add");
    });
}
