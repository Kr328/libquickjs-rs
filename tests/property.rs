use libquickjs::{EvalFlags, GetOwnAtomFlags, Runtime, Value};

#[test]
fn test_enum_property() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    let ret = ctx
        .eval_global(
            None,
            r#"const obj = {a: 1, b: 2, c: 3}; obj"#,
            "script.js",
            EvalFlags::empty(),
        )
        .unwrap();

    let atoms = ctx.get_own_property_atoms(&ret, GetOwnAtomFlags::empty()).unwrap();
    for atom in atoms {
        let key = ctx.atom_to_value(&atom.atom).unwrap();
        let value = ctx.get_property(&ret, &atom.atom).unwrap();

        let key = ctx.get_string(&key).unwrap();
        match &*key {
            "a" => assert!(matches!(value, Value::Int32(1))),
            "b" => assert!(matches!(value, Value::Int32(2))),
            "c" => assert!(matches!(value, Value::Int32(3))),
            _ => panic!("unknown key"),
        }
    }
}
