use libquickjs::{Class, Runtime, Value};

#[test]
fn test_set_get_prototype() {
    struct MyClass;

    impl Class for MyClass {
        const NAME: &'static str = "MyClass";
    }

    let rt = Runtime::new();

    let ctx = rt.new_context();
    let null_prototype = ctx.get_class_proto::<MyClass>();

    assert_eq!(null_prototype, Value::Null);
}
