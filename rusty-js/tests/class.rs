use rusty_js_macro::js_class;
use rustyjs_test::*;

#[js_class]
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn add(&self, p: Point) -> Self {
        Self {
            x: self.x + p.x,
            y: self.y + p.y,
        }
    }

    fn sadd(x: i32, y: i32) -> Self {
        Self { x: x + 1, y: y + 1 }
    }
}

impl JSClass<JSEngineValue> for Point {
    const NAME: &'static str = "Point";

    fn data_constructor() -> Constructor<JSEngineValue> {
        Constructor::new(|x, y| Point { x, y })
    }

    fn class_setup(class: &ClassSetup<JSEngineValue>) -> JSResult<()> {
        // Define instance property with getter and setter
        class.property("x", |builder| {
            let getter = class.new_func(|this: This<Point>| this.x)?;
            let setter = class.new_func(|mut this: ThisMut<Point>, x: i32| this.x = x)?;
            Ok(builder
                .getter(getter)
                .setter(setter)
                .with_default_method_attr())
        })?;

        class.property("y", |builder| {
            let getter = class.new_func(|this: This<Point>| this.y)?;
            let setter = class.new_func(|mut this: ThisMut<Point>, y: i32| this.y = y)?;
            Ok(builder
                .getter(getter)
                .setter(setter)
                .with_default_method_attr())
        })?;

        // Define static property with getter and setter
        class.static_property("origin", |builder| {
            let getter = class.new_func(|| Point { x: 0x5a, y: 0xa5 })?;
            let setter = class.new_func(|| {
                // Read-only property, setter does nothing
            })?;
            Ok(builder.getter(getter).setter(setter).configurable())
        })?;

        class.method("add", |this: This<Point>, p: Point| this.add(p))?;

        // Define static method
        class.static_method("sadd", |x: i32, y: i32| Self::sadd(x, y))?;
        Ok(())
    }
}

#[test]
fn constructor() {
    run(|ctx| {
        ctx.register_class::<Point>()?;
        let point = ctx.eval::<Point>(Source::from_bytes(b"let point=new Point(2,3); point"))?;
        assert_eq!(point.x, 2);
        assert_eq!(point.y, 3);

        // Test instance_of with Point
        let obj = ctx.eval::<JSObject>(Source::from_bytes(b"new Point(2,3)"))?;
        assert!(Class::instance_of::<Point>(&obj));

        // Test instance_of with non-Point object
        let obj = ctx.eval::<JSObject>(Source::from_bytes(b"let o = {}; o"))?;
        assert!(!Class::instance_of::<Point>(&obj));

        assert_eq!(
            ctx.eval::<String>(Source::from_bytes(b"Point.constructor.name"))?,
            "Function"
        );

        assert!(ctx.eval::<bool>(Source::from_bytes(b"Point.prototype.constructor==Point"))?);

        // JSC: it's object currently
        // assert_eq!(
        //     ctx.eval::<String>(Source::from_bytes(b"typeof Point"))?,
        //     "function"
        // );

        assert!(ctx.eval::<bool>(Source::from_bytes(b"point instanceof Point"))?);
        Ok(())
    });
}

#[test]
fn rustfunc_class_registered() {
    run(|ctx| {
        assert_eq!(
            ctx.eval::<String>(Source::from_bytes(b"RustFunc.name"))?,
            "RustFunc"
        );

        assert_eq!(
            ctx.eval::<String>(Source::from_bytes(b"RustFunc.constructor.name"))?,
            "Function"
        );
        Ok(())
    });
}

#[test]
fn basic_add_fn() {
    run(|ctx| {
        let func = JSFunc::new(ctx, |a: i32, b: i32, c: i32| a + b + c)?.name("add")?;
        ctx.global().set("add", func)?;

        assert!(ctx.eval::<JSFunc>(Source::from_bytes(b"add")).is_ok());
        assert_eq!(
            ctx.eval::<i32>(Source::from_bytes(b"add(7, 9,1)")).unwrap(),
            17
        );
        assert_eq!(
            ctx.eval::<i32>(Source::from_bytes(b"add.length")).unwrap(),
            3
        );
        assert_eq!(
            ctx.eval::<String>(Source::from_bytes(b"add.name")).unwrap(),
            "add"
        );
        Ok(())
    });
}

#[test]
fn test_property_getter_setter() {
    run(|ctx| {
        ctx.register_class::<Point>()?;

        // Test getter
        let point = ctx.eval::<Point>(Source::from_bytes(b"let p = new Point(5, 10); p"))?;
        assert_eq!(point.x, 5);

        // Test setter
        let point = ctx
            .eval::<Point>(Source::from_bytes(b"p.x = 15; p"))
            .unwrap();
        assert_eq!(point.x, 15);

        // Test property descriptor
        assert!(ctx
            .eval::<bool>(Source::from_bytes(
                b"Object.getOwnPropertyDescriptor(Point.prototype, 'x').configurable"
            ))
            .unwrap());
        assert!(ctx
            .eval::<bool>(Source::from_bytes(
                b"Object.getOwnPropertyDescriptor(Point.prototype, 'x').get !== undefined"
            ))
            .unwrap());
        assert!(ctx
            .eval::<bool>(Source::from_bytes(
                b"Object.getOwnPropertyDescriptor(Point.prototype, 'x').set !== undefined"
            ))
            .unwrap());
        Ok(())
    });
}

#[test]
fn test_instance_method() {
    run(|ctx| {
        ctx.register_class::<Point>()?;

        // Test method call
        let result = ctx
            .eval::<Point>(Source::from_bytes(
                b"let p1 = new Point(1, 2); let p2 = new Point(3, 4); p1.add(p2)",
            ))
            .unwrap();
        assert_eq!(result.x, 4); // 1 + 3
        assert_eq!(result.y, 6); // 2 + 4

        // Test method exists on prototype
        assert!(ctx
            .eval::<bool>(Source::from_bytes(b"'add' in Point.prototype"))
            .unwrap());
        assert!(ctx
            .eval::<bool>(Source::from_bytes(
                b"typeof Point.prototype.add === 'function'"
            ))
            .unwrap());
        Ok(())
    });
}

#[test]
fn test_static_method() {
    run(|ctx| {
        ctx.register_class::<Point>()?;

        // Test static method call
        let result = ctx
            .eval::<Point>(Source::from_bytes(b"Point.sadd(5, 7)"))
            .unwrap();
        assert_eq!(result.x, 6); // 5 + 1
        assert_eq!(result.y, 8); // 7 + 1

        // Test static method exists on constructor
        assert!(ctx
            .eval::<bool>(Source::from_bytes(b"'sadd' in Point"))
            .unwrap());
        assert!(ctx
            .eval::<bool>(Source::from_bytes(b"typeof Point.sadd === 'function'"))
            .unwrap());
        Ok(())
    });
}

#[test]
fn test_static_property() {
    run(|ctx| {
        ctx.register_class::<Point>()?;

        // Test static getter
        let origin = ctx
            .eval::<Point>(Source::from_bytes(b"Point.origin"))
            .unwrap();
        assert_eq!(origin.x, 0x5a);
        assert_eq!(origin.y, 0xa5);

        // Test property exists on constructor
        assert!(ctx
            .eval::<bool>(Source::from_bytes(b"'origin' in Point"))
            .unwrap());

        // Test property descriptor
        assert!(ctx
            .eval::<bool>(Source::from_bytes(
                b"Object.getOwnPropertyDescriptor(Point, 'origin').configurable"
            ))
            .unwrap());
        assert!(ctx
            .eval::<bool>(Source::from_bytes(
                b"Object.getOwnPropertyDescriptor(Point, 'origin').get !== undefined"
            ))
            .unwrap());
        assert!(ctx
            .eval::<bool>(Source::from_bytes(
                b"Object.getOwnPropertyDescriptor(Point, 'origin').set !== undefined"
            ))
            .unwrap());

        // Test property is not on prototype or instances
        assert!(!ctx
            .eval::<bool>(Source::from_bytes(b"'origin' in Point.prototype"))
            .unwrap());
        assert!(!ctx
            .eval::<bool>(Source::from_bytes(b"'origin' in (new Point(0, 0))"))
            .unwrap());
        Ok(())
    });
}

#[test]
fn test_extend_class() {
    run(|ctx| {
        ctx.register_class::<Point>()?;

        // Test class extension with method inheritance
        let result = ctx
            .eval::<i32>(Source::from_bytes(
                br#"

                class ColorPoint extends Point {
                    constructor(x, y, color) {
                        super(x, y);
                        this.color = color;
                    }
                    get_color() {
                        return this.color;
                    }
                }
                let p = new ColorPoint(2, 3, 0x5fa5);

                // Verify prototype chain
                if (!(ColorPoint.prototype.__proto__ === Point.prototype)) {
                    throw new Error('Prototype chain broken');
                }
                if (!(ColorPoint.__proto__ === Point)) {
                    throw new Error('Constructor chain broken');
                }

                // Verify inherited methods work
                let added = p.add(new Point(1, 2));
                if (added.x !== 3 || added.y !== 5) {
                     throw new Error('Inherited method failed');
                }

                // Verify new method works
                p.get_color()
                "#,
            ))
            .unwrap();
        assert_eq!(result, 0x5fa5);
        Ok(())
    });
}
