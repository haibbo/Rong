mod helper;
use helper::*;

#[test]
fn test_object() {
    run(|ctx| {
        let v = 3;
        let key = JSValue::from(ctx, "key");
        let value = JSValue::from(ctx, v);

        let obj = JSObject::new(ctx);

        assert!(obj.set(key.clone(), value));
        assert!(obj.has(key.clone()));

        let val = obj.get(key.clone());
        assert_eq!(val.try_into::<i32>().unwrap(), v);

        obj.del(key.clone());
        assert!(!obj.has(key.clone()));
    });
}
