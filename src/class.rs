use crate::{Context, GlobalValue, Runtime, value::Value};

#[derive(Copy, Clone)]
pub struct CallOptions {
    pub constructor: bool,
}

pub trait GCMarker {
    fn mark_value(&self, value: &Value);
    fn mark_global_value(&self, value: &GlobalValue);
}

pub trait Class: Send + 'static {
    const NAME: &'static str;

    fn call<'rt>(
        &self,
        ctx: &Context<'rt>,
        func: &Value,
        this: &Value,
        args: &[Value],
        options: CallOptions,
    ) -> Result<Value<'rt>, Value<'rt>>;

    fn gc_mark<M: GCMarker>(&self, marker: &M) {
        let _ = marker;
    }

    fn on_registered(rt: &Runtime) {
        let _ = rt;
    }
}
