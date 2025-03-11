use libquickjs::{EvalFlags, NativeFunction, PropertyDescriptorFlags, Runtime, Value};

#[test]
fn test_call_native_func() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    let global_obj = ctx.get_global_object();
    let func = ctx
        .new_object_class(
            NativeFunction::new(|ctx, _, _, args, _| {
                let name = if args.is_empty() {
                    return Err(ctx.new_string("invalid argument")?);
                } else {
                    ctx.get_string(&args[0])?
                };

                println!("hello {}", &*name);

                Ok(Value::Undefined)
            }),
            None,
        )
        .unwrap();
    ctx.define_property_value_str(
        &global_obj,
        "hello",
        func,
        PropertyDescriptorFlags::CONFIGURABLE | PropertyDescriptorFlags::WRITABLE | PropertyDescriptorFlags::ENUMERABLE,
    )
    .unwrap();

    ctx.eval_global(None, "hello('world!!')", "test.js", EvalFlags::STRICT)
        .unwrap();
}
