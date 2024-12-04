mod helper;
use helper::*;

#[test]
fn test_object() {
    run(|ctx| {
        let v = 3;
        let key = "key";
        let obj = JSObject::new(ctx);

        assert!(obj.set(key, v));
        assert!(obj.has(key));

        let val = obj.get(key);
        assert_eq!(val.try_into::<i32>().unwrap(), v);

        obj.del(key);
        assert!(!obj.has(key));

        let value = JSValue::from(ctx, v);
        assert!(obj.set(key, value.clone()));
        assert_eq!(obj.get(key).try_into::<i32>().unwrap(), v);

        assert!(obj.set(9, value.clone()));
        assert_eq!(obj.get(9).try_into::<i32>().unwrap(), v);

        let objv = JSObject::new(ctx);
        assert!(obj.set("obj", objv));
        assert!(obj.has("obj"));
    });
}
