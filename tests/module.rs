use libquickjs::{EvalFlags, PromiseState, Runtime, Value};

#[test]
fn test_simple_module() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    let ret = ctx
        .eval_module(
            r#"
        export const a = 114514;
        export const b = 1919810;
        "#,
            "module.js",
            EvalFlags::empty(),
        )
        .unwrap();

    assert_eq!(ctx.get_promise_state(&ret).unwrap(), PromiseState::Fulfilled);

    let ret = ctx
        .eval_module(
            "import { a, b } from 'module.js'; globalThis.result = a + b;",
            "script.js",
            EvalFlags::empty(),
        )
        .unwrap();

    assert_eq!(ctx.get_promise_state(&ret).unwrap(), PromiseState::Fulfilled);

    let global_obj = ctx.get_global_object();
    let ret = ctx.get_property_str(&global_obj, "result").unwrap();

    match ret {
        Value::Int32(v) => {
            assert_eq!(v, 114514 + 1919810);
        }
        _ => panic!("unexpected return type: {:?}", ret),
    }
}
