use crate::{
    Context,
    class::{CallOptions, Class},
    value::Value,
};

#[derive(Clone)]
#[repr(transparent)]
pub struct NativeFunction<F>
where
    F: for<'rt> Fn(&Context<'rt>, &Value, &Value, &[Value], CallOptions) -> Result<Value<'rt>, Value<'rt>> + Send + 'static,
{
    func: F,
}

impl<F> NativeFunction<F>
where
    F: for<'rt> Fn(&Context<'rt>, &Value, &Value, &[Value], CallOptions) -> Result<Value<'rt>, Value<'rt>> + Send + 'static,
{
    pub const fn new(func: F) -> Self {
        Self { func }
    }
}

impl<F> Class for NativeFunction<F>
where
    F: for<'rt> Fn(&Context<'rt>, &Value, &Value, &[Value], CallOptions) -> Result<Value<'rt>, Value<'rt>> + Send + 'static,
{
    const NAME: &'static str = "NativeFunction";

    fn call<'rt>(
        &self,
        ctx: &Context<'rt>,
        func: &Value,
        this: &Value,
        args: &[Value],
        options: CallOptions,
    ) -> Result<Value<'rt>, Value<'rt>> {
        (self.func)(ctx, func, this, args, options)
    }
}

pub trait NativeFunctionExt<'rt> {
    fn define_native_function<F>(self, obj: &Value, name: &str, func: F) -> Result<bool, Value<'rt>>
    where
        F: for<'r> Fn(&Context<'r>, &Value, &Value, &[Value], CallOptions) -> Result<Value<'r>, Value<'r>> + Send + 'static;
}

impl<'rt> NativeFunctionExt<'rt> for Context<'rt> {
    fn define_native_function<F>(self, obj: &Value, name: &str, func: F) -> Result<bool, Value<'rt>>
    where
        F: for<'r> Fn(&Context<'r>, &Value, &Value, &[Value], CallOptions) -> Result<Value<'r>, Value<'r>> + Send + 'static,
    {
        let func = NativeFunction::new(func);
        self.define_property_value_str(obj, &name, self.new_object_class(func, None)?, Default::default())
    }
}
