use crate::{
    class::{CallOptions, Class},
    value::{Object, Value},
    Context,
};

pub struct NativeFunction<F>
where
    F: for<'rt> Fn(&Context<'rt>, &Object, &Value, &[Value], CallOptions) -> Result<Value<'rt>, Value<'rt>> + Send + 'static,
{
    func: F,
}

impl<F> NativeFunction<F>
where
    F: for<'rt> Fn(&Context<'rt>, &Object, &Value, &[Value], CallOptions) -> Result<Value<'rt>, Value<'rt>> + Send + 'static,
{
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

impl<F> Class for NativeFunction<F>
where
    F: for<'rt> Fn(&Context<'rt>, &Object, &Value, &[Value], CallOptions) -> Result<Value<'rt>, Value<'rt>> + Send + 'static,
{
    const NAME: &'static str = "NativeFunction";

    fn call<'rt>(
        &self,
        ctx: &Context<'rt>,
        func: &Object,
        this: &Value,
        args: &[Value],
        options: CallOptions,
    ) -> Result<Value<'rt>, Value<'rt>> {
        (self.func)(ctx, func, this, args, options)
    }
}
