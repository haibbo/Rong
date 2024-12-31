mod helper;
use helper::*;

#[test]
fn basic() {
    run(|ctx| {
        let v = 3;
        let key = "key";
        let obj = JSObject::new(ctx);
        assert_some!(obj.is_object());

        assert!(obj.set(key, v));
        assert!(obj.has(key));

        obj.del(key);
        assert!(!obj.has(key));

        let value = JSValue::from(ctx, v);

        // JSValue as Property Value
        assert!(obj.set(key, value.clone()));
        assert_eq!(obj.get::<&str, i32>(key).unwrap(), v);
        assert!(obj.set(9, value.clone()));
        assert_eq!(obj.get::<i32, i32>(9).unwrap(), v);

        let objv = JSObject::new(ctx);
        assert!(obj.set("obj", objv));
        assert!(obj.has("obj"));
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
        assert_some!(obj.get::<_, JSObject>("a1").unwrap().is_array());
        assert_some!(obj.get::<_, JSObject>("a2").unwrap().is_array());
        assert_some!(obj.get::<_, JSObject>("a3").unwrap().is_array());
        assert_some!(obj.get::<_, JSObject>("func1").unwrap().is_function());
        assert_some!(obj.get::<_, JSObject>("func2").unwrap().is_function());
        assert_none!(obj.get::<_, JSObject>("obj1").unwrap().is_function());
        assert_none!(obj.get::<_, JSObject>("obj1").unwrap().is_array());
        assert_some!(obj.get::<_, JSObject>("obj1").unwrap().is_object());
        assert_eq!(
            obj.get::<_, JSObject>("obj1")
                .unwrap()
                .get::<_, i32>("a")
                .unwrap(),
            1
        );

        let result: Result<String, String> = obj.get("None");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Property not found");
    })
}
