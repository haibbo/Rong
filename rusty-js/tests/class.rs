mod helper;
use helper::*;

struct Point {
    x: i32,
    y: i32,
}

impl IntoJSValue<JSEngineValue> for Point {
    fn into_js_value(self, context: &JSEngineContext) -> JSEngineValue {
        JSEngineValue::from((context, 1))
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
        ctx.eval::<()>("new Point(2,3)").unwrap();
    });
}
