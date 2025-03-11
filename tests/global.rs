use libquickjs::Runtime;

#[test]
fn test_global_objects() {
    let rt = Runtime::new();

    let ctx = rt.new_context();
    let obj = ctx.new_object(None).unwrap();

    let global_obj = rt.new_global_value(&obj).unwrap();
    let _ = global_obj;
}
