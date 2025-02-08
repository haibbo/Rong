use rusty_js_macro::{js_class, js_method, js_methods};
use rustyjs_test::*;
use std::sync::{Mutex, OnceLock};
use tokio::time::Duration;

#[js_class(rename = "PointX")]
#[derive(Debug, PartialEq)]
pub struct Point {
    x: i32,
    y: i32,
}

static ORIGIN: OnceLock<Mutex<Point>> = OnceLock::new();

#[js_methods]
impl Point {
    #[js_method(constructor)]
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    #[js_method(getter, configurable, rename = "ORIGIN")]
    fn get_origin() -> Point {
        ORIGIN
            .get_or_init(|| Mutex::new(Point { x: 0x5a, y: 0xa5 }))
            .lock()
            .unwrap()
            .clone()
    }

    #[js_method(setter, rename = "ORIGIN")]
    fn set_origin(point: Point) {
        if let Some(origin) = ORIGIN.get() {
            *origin.lock().unwrap() = point;
        }
    }

    #[js_method(getter, enumerable, rename = "x")]
    fn getx(&self) -> i32 {
        self.x
    }

    #[js_method(setter, rename = "x")]
    fn setx(&mut self, x: i32) {
        self.x = x;
    }

    #[js_method(getter, rename = "y")]
    fn gety(&self) -> i32 {
        self.y
    }

    #[js_method(setter, rename = "y")]
    fn sety(&mut self, y: i32) {
        self.y = y;
    }

    #[js_method(rename = "Add")]
    pub fn add(&self, other: Point) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }

    #[js_method(rename = "create")]
    pub fn create(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    #[js_method(rename = "moveBy")]
    pub fn move_by(&mut self, dx: i32, dy: i32) {
        self.x += dx;
        self.y += dy;
    }

    #[js_method(rename = "moveByAsync")]
    pub async fn move_by_async(&mut self, dx: i32, dy: i32) {
        tokio::time::sleep(Duration::from_millis(50)).await;
        self.x += dx;
        self.y += dy;
    }

    #[js_method(rename = "createAsync")]
    pub async fn create_async(x: i32, y: i32) -> Self {
        tokio::time::sleep(Duration::from_millis(50)).await;
        Self { x, y }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> JSContext {
        let rt = RustyJS::runtime();
        let ctx = RustyJS::context(&rt);
        ctx.register_class::<Point>();
        ctx
    }

    #[test]
    fn test_constructor() {
        let ctx = setup();
        let point: Point = ctx.eval(Source::from_bytes("new PointX(2, 3)")).unwrap();
        assert_eq!(point, Point { x: 2, y: 3 });
    }

    #[test]
    fn test_instance_property() {
        let ctx = setup();

        // Test value behavior
        let result: i32 = ctx
            .eval(Source::from_bytes(
                r#"
                let p = new PointX(1, 2);
                p.x = 10;
                p.y = 20;
                p.x + p.y
                "#,
            ))
            .unwrap();
        assert_eq!(result, 30);

        // Test property attributes
        let desc: bool = ctx
            .eval(Source::from_bytes(
                r#"
                p = new PointX(1, 2);
                let desc = Object.getOwnPropertyDescriptor(p.constructor.prototype, 'x');
                desc.configurable === true &&  // 添加 configurable 测试
                desc.enumerable === true &&
                typeof desc.get === 'function' &&
                typeof desc.set === 'function'
                "#,
            ))
            .unwrap();
        assert!(
            desc,
            "x should be configurable and enumerable with accessors"
        );

        // Test non-enumerable property
        let desc: bool = ctx
            .eval(Source::from_bytes(
                r#"
                desc = Object.getOwnPropertyDescriptor(p.constructor.prototype, 'y');
                desc.configurable === true &&  // 添加 configurable 测试
                desc.enumerable === false &&
                typeof desc.get === 'function' &&
                typeof desc.set === 'function'
                "#,
            ))
            .unwrap();
        assert!(
            desc,
            "y should be configurable but not enumerable with accessors"
        );
    }

    #[test]
    fn test_instance_method() {
        let ctx = setup();
        let result: Point = ctx
            .eval(Source::from_bytes(
                r#"
            let p1 = new PointX(1, 2);
            let p2 = new PointX(3, 4);
            p1.Add(p2)
        "#,
            ))
            .unwrap();
        assert_eq!(result, Point { x: 4, y: 6 });
    }

    #[test]
    fn test_static_method() {
        let ctx = setup();
        let result: Point = ctx.eval(Source::from_bytes("PointX.create(5, 6)")).unwrap();
        assert_eq!(result, Point { x: 5, y: 6 });
    }

    #[test]
    fn test_mutable_instance_method() {
        let ctx = setup();
        let result: Point = ctx
            .eval(Source::from_bytes(
                r#"
                let p = new PointX(1, 2);
                p.moveBy(10, 20);
                p
            "#,
            ))
            .unwrap();
        assert_eq!(result, Point { x: 11, y: 22 });
    }

    #[test]
    fn test_static_property() {
        let ctx = setup();

        // Test value behavior
        let result: Point = ctx.eval(Source::from_bytes("PointX.ORIGIN")).unwrap();
        assert_eq!(result, Point { x: 0x5a, y: 0xa5 });

        // Test setting new value
        let result: Point = ctx
            .eval(Source::from_bytes(
                r#"
                PointX.ORIGIN = new PointX(1, 2);
                PointX.ORIGIN
            "#,
            ))
            .unwrap();
        assert_eq!(result, Point { x: 1, y: 2 });

        // Test property attributes
        let desc: bool = ctx
            .eval(Source::from_bytes(
                r#"
                let desc = Object.getOwnPropertyDescriptor(PointX, 'ORIGIN');
                desc.configurable === true &&
                desc.enumerable === false &&
                typeof desc.get === 'function' &&
                typeof desc.set === 'function'
                "#,
            ))
            .unwrap();
        assert!(
            desc,
            "ORIGIN should be configurable but not enumerable, with getter and setter"
        );
    }

    #[test]
    fn test_async_instance_method() {
        async_run!(|ctx: JSContext| async move {
            ctx.register_class::<Point>();

            let result: Point = ctx
                .eval_async(Source::from_bytes(
                    r#"
                    (async function() {
                        let p = new PointX(1, 2);
                        await p.moveByAsync(10, 20);
                        return p;
                    })();
                "#,
                ))
                .await?;
            assert_eq!(result, Point { x: 11, y: 22 });
            Ok(())
        });
    }

    #[test]
    fn test_async_static_method() {
        async_run!(|ctx: JSContext| async move {
            ctx.register_class::<Point>();

            let result: Point = ctx
                .eval_async(Source::from_bytes(
                    r#"
                    (async function() {
                        return await PointX.createAsync(5, 6);
                    })();
                "#,
                ))
                .await?;
            assert_eq!(result, Point { x: 5, y: 6 });
            Ok(())
        });
    }
}
