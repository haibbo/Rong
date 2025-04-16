use rong_test::*;

#[test]
fn basic() {
    run(|ctx| {
        let v = 3;
        let key = "key";
        let obj = JSObject::new(ctx);
        assert!(obj.is_object());

        assert!(obj.set(key, v).is_ok());
        assert!(obj.has(key));

        obj.del(key);
        assert!(!obj.has(key));

        let value = JSValue::from(ctx, v);

        // JSValue as Property Value
        assert!(obj.set(key, value.clone()).is_ok());
        assert_eq!(obj.get::<&str, i32>(key).unwrap(), v);
        assert!(obj.set(9, value.clone()).is_ok());
        assert_eq!(obj.get::<i32, i32>(9).unwrap(), v);

        let objv = JSObject::new(ctx);
        assert!(obj.set("obj", objv).is_ok());
        assert!(obj.has("obj"));
        Ok(())
    });
}

#[test]
fn from_javascript() {
    run(|ctx| {
        let obj: JSObject = ctx
            .eval(Source::from_bytes(
                br#"
                let a3 = [];
                a3[1] = "foo";
                ({
                    a1: [0,,2,,5],
                    a2: [0,"michael",{},undefined,5],
                    a3: a3,
                    func1: () => 1,
                    func2: function(){ return "bar"},
                    obj1: {
                        a: 1,
                        b: "foo",
                    },
                })
                "#,
            ))
            .unwrap();
        assert!(obj.get::<_, JSObject>("a1").unwrap().is_array());
        assert!(obj.get::<_, JSObject>("a2").unwrap().is_array());
        assert!(obj.get::<_, JSObject>("a3").unwrap().is_array());
        assert!(obj.get::<_, JSObject>("func1").unwrap().is_function());
        assert!(obj.get::<_, JSObject>("func2").unwrap().is_function());
        assert!(!obj.get::<_, JSObject>("obj1").unwrap().is_function());
        assert!(!obj.get::<_, JSObject>("obj1").unwrap().is_array());
        assert!(obj.get::<_, JSObject>("obj1").unwrap().is_object());
        assert_eq!(
            obj.get::<_, JSObject>("obj1")
                .unwrap()
                .get::<_, i32>("a")
                .unwrap(),
            1
        );

        let result: Result<String, RongJSError> = obj.get("None");
        assert!(result.is_err());
        match result {
            Err(RongJSError::PropertyNotFound(_)) => (),
            _ => panic!("Expected PropertyNotFound error"),
        }
        Ok(())
    })
}

#[test]
fn test_object_display() {
    run(|ctx| {
        // Test object with properties
        let code = "({foo: 'bar', num: 42})";
        let obj: JSObject = ctx.eval(Source::from_bytes(code)).unwrap();
        assert_eq!(format!("{}", obj), "object");
        assert_eq!(format!("{:?}", obj), "JSObject(object)");

        // Test array object
        let code = "[1, 2, 3]";
        let obj: JSObject = ctx.eval(Source::from_bytes(code)).unwrap();
        assert_eq!(format!("{}", obj), "array");
        assert_eq!(format!("{:?}", obj), "JSObject(array)");
        Ok(())
    });
}

#[test]
fn test_object_properties() {
    run(|ctx| {
        // Create a test object
        let source = Source::from_bytes(
            r#"
            let obj = {
                name: "test",
                age: 42,
                greet: function() { return "Hello"; }
            };
            obj;
            "#,
        );

        let obj: JSObject = ctx.eval(source).unwrap();

        // Test basic property operations
        assert_eq!(obj.get::<_, String>("name").unwrap(), "test");
        assert_eq!(obj.get::<_, i32>("age").unwrap(), 42);
        assert!(obj.has("greet"));

        // Test entries
        let entries = obj.entries().unwrap();
        assert_eq!(entries.len(), 3);

        // Test typed entries with flexible value types
        let entries: Vec<(String, JSValue)> = obj.entries_as().unwrap();
        assert!(entries.iter().any(|(k, _)| k == "name"));

        // Test keys
        let keys: Vec<String> = obj.keys_as().unwrap();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&"name".to_string()));
        assert!(keys.contains(&"age".to_string()));
        assert!(keys.contains(&"greet".to_string()));

        // Test values
        let values = obj.values().unwrap();
        assert_eq!(values.count(), 3);

        // Test property modification
        assert!(obj.set("name", "updated").is_ok());
        assert_eq!(obj.get::<_, String>("name").unwrap(), "updated");

        // Test property deletion
        assert!(obj.del("age"));
        assert!(!obj.has("age"));

        // Test non-existent property
        assert!(obj.get::<_, String>("nonexistent").is_err());
        Ok(())
    });
}

#[test]
fn test_object_property_attributes() {
    run(|ctx| {
        // Test property attributes
        let source = Source::from_bytes(
            r#"
            let obj = {};
            Object.defineProperty(obj, 'readOnly', {
                value: 'constant',
                writable: false,
                enumerable: true
            });
            Object.defineProperty(obj, 'hidden', {
                value: 'secret',
                enumerable: false
            });
            obj;
            "#,
        );

        let obj: JSObject = ctx.eval(source).unwrap();

        // Test read-only property
        assert_eq!(obj.get::<_, String>("readOnly").unwrap(), "constant");

        // Test non-enumerable property
        let keys: Vec<String> = obj.keys_as().unwrap();
        assert!(keys.contains(&"readOnly".to_string()));
        assert!(!keys.contains(&"hidden".to_string()));

        // Test property existence
        assert!(obj.has("hidden"));
        assert!(obj.has("readOnly"));
        Ok(())
    });
}

#[test]
fn test_object_prototype() {
    run(|ctx| {
        // Test prototype chain
        let source = Source::from_bytes(
            r#"
            function Animal(name) {
                this.name = name;
            }
            Animal.prototype.speak = function() {
                return this.name + " makes a sound";
            };

            let dog = new Animal("Dog");
            dog;
            "#,
        );

        let obj: JSObject = ctx.eval(source).unwrap();

        // Test instance property
        assert_eq!(obj.get::<_, String>("name").unwrap(), "Dog");

        // Test prototype method
        let result: String = ctx
            .eval(Source::from_bytes(
                r#"
                dog.speak();
                "#,
            ))
            .unwrap();
        assert_eq!(result, "Dog makes a sound");

        // Test own properties
        let own_keys: Vec<String> = obj.keys_as().unwrap();
        assert!(own_keys.contains(&"name".to_string()));
        assert!(!own_keys.contains(&"speak".to_string())); // speak is on the prototype
        Ok(())
    });
}
