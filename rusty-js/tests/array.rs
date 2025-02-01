mod helper;
use helper::*;

#[test]
fn test_array_basic_operations() {
    run(|ctx| {
        // Create new array
        let array = ctx
            .eval::<JSArray>(Source::from_bytes("let arr = [1, 2, 3]; arr"))
            .unwrap();

        // Test length
        assert_eq!(array.len(), 3);

        // Test get
        let first: i32 = array.get(0).unwrap();
        assert_eq!(first, 1);

        // Test set
        array.set(0, 10).unwrap();
        let first: i32 = array.get(0).unwrap();
        assert_eq!(first, 10);

        // Test is_empty
        assert!(!array.is_empty());
    });
}

#[test]
fn test_array_iteration() {
    run(|ctx| {
        // Create array from JavaScript
        let array = ctx
            .eval::<JSArray>(Source::from_bytes("[10, 20, 30]"))
            .unwrap();

        // Test iterator
        let mut sum = 0;
        for item in array.iter::<i32>() {
            sum += item.unwrap();
        }
        assert_eq!(sum, 60);

        // Test exact size iterator
        let iter = array.iter::<i32>();
        assert_eq!(iter.len(), 3);
    });
}
