use libquickjs::{EvalFlags, Runtime, Value};

#[test]
fn test_return_int() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    let ret = ctx.eval_global(None, "114514", "script.js", EvalFlags::empty()).unwrap();

    match ret {
        Value::Int32(v) => {
            assert_eq!(v, 114514);
        }
        _ => panic!("unexpected return type: {:?}", ret),
    }
}

#[test]
fn test_return_string() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    let ret = ctx.eval_global(None, r#""114514""#, "script.js", EvalFlags::empty()).unwrap();

    match ret {
        Value::String(v) => {
            let s = ctx.get_string(&v).unwrap();
            assert_eq!(&*s, "114514");
        }
        _ => panic!("unexpected return type: {:?}", ret),
    }
}
