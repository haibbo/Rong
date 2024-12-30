mod helper;
use helper::*;
use rusty_js_core::function::{Optional, RegularTypeSealed, Rest, This, ThisMut};

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

impl RegularTypeSealed for Point {}

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

    fn data_constructor() -> RustFunc {
        RustFunc::new(|x, y| Point { x, y })
    }

    fn class_setup(class: &ClassSetup<JSEngineValue>) {
        // Define instance property with getter and setter
        class.property("x", |builder| {
            let getter = class.new_func(|this: This<Point>| this.x);
            let setter = class.new_func(|mut this: ThisMut<Point>, x: i32| this.x = x);
            builder
                .getter(getter)
                .setter(setter)
                .with_default_method_attr()
        });

        // Define static property with getter and setter
        class.static_property("origin", |builder| {
            let getter = class.new_func(|| Point { x: 0, y: 0 });
            let setter = class.new_func(|_p: Point| {
                // Read-only property, setter does nothing
            });
            builder.getter(getter).setter(setter).configurable()
        });

        // Define instance method
        class.method("add", |this: This<Point>, p: Point| this.add(p));

        // Define static method
        class.static_method("sadd", |x: i32, y: i32| Self::sadd(x, y));
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

#[test]
fn test_property_getter_setter() {
    run(|ctx| {
        ctx.register_class::<Point>();

        // Test getter
        let point = ctx.eval::<Point>("let p = new Point(5, 10); p").unwrap();
        assert_eq!(point.x, 5);

        // Test setter
        let point = ctx.eval::<Point>("p.x = 15; p").unwrap();
        assert_eq!(point.x, 15);

        // Test property descriptor
        assert!(ctx
            .eval::<bool>("Object.getOwnPropertyDescriptor(Point.prototype, 'x').configurable")
            .unwrap());
        assert!(ctx
            .eval::<bool>("Object.getOwnPropertyDescriptor(Point.prototype, 'x').get !== undefined")
            .unwrap());
        assert!(ctx
            .eval::<bool>("Object.getOwnPropertyDescriptor(Point.prototype, 'x').set !== undefined")
            .unwrap());
    });
}

#[test]
fn test_instance_method() {
    run(|ctx| {
        ctx.register_class::<Point>();

        // Test method call
        let result = ctx
            .eval::<Point>("let p1 = new Point(1, 2); let p2 = new Point(3, 4); p1.add(p2)")
            .unwrap();
        assert_eq!(result.x, 4); // 1 + 3
        assert_eq!(result.y, 6); // 2 + 4

        // Test method exists on prototype
        assert!(ctx.eval::<bool>("'add' in Point.prototype").unwrap());
        assert!(ctx
            .eval::<bool>("typeof Point.prototype.add === 'function'")
            .unwrap());
    });
}

#[test]
fn test_static_method() {
    run(|ctx| {
        ctx.register_class::<Point>();

        // Test static method call
        let result = ctx.eval::<Point>("Point.sadd(5, 7)").unwrap();
        assert_eq!(result.x, 6); // 5 + 1
        assert_eq!(result.y, 8); // 7 + 1

        // Test static method exists on constructor
        assert!(ctx.eval::<bool>("'sadd' in Point").unwrap());
        assert!(ctx
            .eval::<bool>("typeof Point.sadd === 'function'")
            .unwrap());
    });
}

#[test]
fn test_static_property() {
    run(|ctx| {
        ctx.register_class::<Point>();

        // Test static getter
        let origin = ctx.eval::<Point>("Point.origin").unwrap();
        assert_eq!(origin.x, 0);
        assert_eq!(origin.y, 0);

        // Test property exists on constructor
        assert!(ctx.eval::<bool>("'origin' in Point").unwrap());

        // Test property descriptor
        assert!(ctx
            .eval::<bool>("Object.getOwnPropertyDescriptor(Point, 'origin').configurable")
            .unwrap());
        assert!(ctx
            .eval::<bool>("Object.getOwnPropertyDescriptor(Point, 'origin').get !== undefined")
            .unwrap());
        assert!(ctx
            .eval::<bool>("Object.getOwnPropertyDescriptor(Point, 'origin').set !== undefined")
            .unwrap());

        // Test property is not on prototype or instances
        assert!(!ctx.eval::<bool>("'origin' in Point.prototype").unwrap());
        assert!(!ctx.eval::<bool>("'origin' in (new Point(0, 0))").unwrap());
    });
}
