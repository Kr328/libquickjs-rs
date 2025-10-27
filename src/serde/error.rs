use crate::{Atom, Context, Value};

pub fn error_to_string<'rt>(ctx: &Context, err: &Value) -> String {
    ctx.to_string(err)
        .and_then(|s| ctx.get_string(&s).map(|s| s.to_string()))
        .unwrap_or_else(|_| "internal error".to_string())
}

pub fn collect_path<'a, T, K: FnMut(T) -> Option<&'a Atom<'a>>, C: IntoIterator<Item = T>>(
    ctx: &Context,
    mut k: K,
    holders: C,
) -> Vec<String> {
    let mut path = Vec::new();
    for holder in holders {
        if let Some(key) = k(holder) {
            if let Ok(s) = ctx.atom_to_string(key).and_then(|v| Ok(ctx.get_string(&v)?.to_string())) {
                path.push(s.to_string());
            } else {
                path.push("<unknown>".to_string());
            }
        }
    }
    path.reverse();
    path
}
