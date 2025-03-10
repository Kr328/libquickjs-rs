use libquickjs::{ReadObjectFlags, Runtime, WriteObjectFlags};

#[test]
fn test_write_read_object() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    let obj = ctx.new_object(None).unwrap();
    ctx.set_property_str(&obj, "foo", ctx.new_string("bar").unwrap().into())
        .unwrap();

    let data = ctx.write_object(&obj, WriteObjectFlags::empty()).unwrap();
    println!("data.len: {}", data.len());

    let obj = ctx.read_object(&data, ReadObjectFlags::empty()).unwrap();
    let foo = ctx.get_property_str(&obj, "foo").unwrap();
    let foo_str = ctx.get_string(&foo).unwrap();
    assert_eq!(&*foo_str, "bar");
}
