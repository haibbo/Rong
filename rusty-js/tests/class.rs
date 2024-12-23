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
fn basic() {
    run(|ctx| {
        ctx.register_class::<Point>();
        let point = ctx.eval::<Point>("let t=new Point(2,3);t").unwrap();
        assert_eq!(point.x, 2);
        assert_eq!(point.y, 3);
    });
}
