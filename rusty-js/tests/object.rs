mod helper;
use helper::*;

#[test]
fn test_object() {
    run(|ctx| {
        let v = 3;
        let key = "key";
        let value = JSValue::from(ctx, v);

        let obj = JSObject::new(ctx);

        assert!(obj.set(key, value));
        assert!(obj.has(key));

        let val = obj.get(key);
        assert_eq!(val.try_into::<i32>().unwrap(), v);

        obj.del(key);
        assert!(!obj.has(key));
    });
}
