mod helper;
use helper::*;

struct Point {
    x: i32,
    y: i32,
}

impl IntoJSValue<EJSValue> for Point {
    fn into_js_value(self, context: &EJSContext) -> EJSValue {
        EJSValue::from((context, 1))
    }
}

impl JSClass<EJSValue> for Point {
    const NAME: &'static str = "Point";

    fn data_constructor() -> RustFunc<EJSValue> {
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
