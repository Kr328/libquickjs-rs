use libquickjs::{value::Value, Runtime};

fn main() {
    let rt = Runtime::new();

    let ctx = rt.new_context();
    let ret = ctx.eval_global("const obj = {}; obj", "app.js", Default::default());
    match ret {
        Value::BigDecimal(_) => {
            println!("BigDecimal")
        }
        Value::BigInt(_) => {
            println!("BigInt")
        }
        Value::BigFloat(_) => {
            println!("BigFloat")
        }
        Value::Symbol(_) => {
            println!("Symbol")
        }
        Value::String(_) => {
            println!("String")
        }
        Value::Module(_) => {
            println!("Module")
        }
        Value::FunctionByteCode(_) => {
            println!("FunctionByteCode")
        }
        Value::Object(_) => {
            println!("Object")
        }
        Value::Int32(_) => {
            println!("Int32")
        }
        Value::Bool(_) => {
            println!("Bool")
        }
        Value::Null => {
            println!("Null")
        }
        Value::Undefined => {
            println!("Undefined")
        }
        Value::Uninitialized => {
            println!("Uninitialized")
        }
        Value::CatchOffset(_) => {
            println!("CatchOffset")
        }
        Value::Exception => {
            println!("Exception")
        }
        Value::Float64(_) => {
            println!("Float64")
        }
    }
}
