use crate::{Context, Value};

pub fn error_to_string<'rt>(ctx: &Context, err: &Value) -> String {
    ctx.to_string(err)
        .and_then(|s| ctx.get_string(&s).map(|s| s.to_string()))
        .unwrap_or_else(|_| "internal error".to_string())
}
