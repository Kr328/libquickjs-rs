use crate::{CallOptions, Context, NativeFunction, PropertyDescriptorFlags, Value};

#[derive(Default, Clone)]
pub struct NativeProperty<
    'rt,
    G = for<'r> fn(&Context<'r>, &Value, &Value, &[Value], CallOptions) -> Result<Value<'r>, Value<'r>>,
    S = for<'r> fn(&Context<'r>, &Value, &Value, &[Value], CallOptions) -> Result<Value<'r>, Value<'r>>,
> where
    G: for<'r> Fn(&Context<'r>, &Value, &Value, &[Value], CallOptions) -> Result<Value<'r>, Value<'r>> + Send + 'static,
    S: for<'r> Fn(&Context<'r>, &Value, &Value, &[Value], CallOptions) -> Result<Value<'r>, Value<'r>> + Send + 'static,
{
    pub value: Value<'rt>,
    pub getter: Option<NativeFunction<G>>,
    pub setter: Option<NativeFunction<S>>,
    pub flags: PropertyDescriptorFlags,
}

pub trait NativePropertyExt<'rt> {
    fn define_native_property<'a, G, S>(
        &self,
        obj: &Value,
        name: &str,
        prop: NativeProperty<'a, G, S>,
    ) -> Result<bool, Value<'rt>>
    where
        G: for<'r> Fn(&Context<'r>, &Value, &Value, &[Value], CallOptions) -> Result<Value<'r>, Value<'r>> + Send + 'static,
        S: for<'r> Fn(&Context<'r>, &Value, &Value, &[Value], CallOptions) -> Result<Value<'r>, Value<'r>> + Send + 'static;
}

impl<'rt> NativePropertyExt<'rt> for Context<'rt> {
    fn define_native_property<'a, G, S>(
        &self,
        obj: &Value,
        name: &str,
        prop: NativeProperty<'a, G, S>,
    ) -> Result<bool, Value<'rt>>
    where
        G: for<'r> Fn(&Context<'r>, &Value, &Value, &[Value], CallOptions) -> Result<Value<'r>, Value<'r>> + Send + 'static,
        S: for<'r> Fn(&Context<'r>, &Value, &Value, &[Value], CallOptions) -> Result<Value<'r>, Value<'r>> + Send + 'static,
    {
        let atom = self.new_atom(name)?;
        let getter = match prop.getter {
            Some(getter) => self.new_object_class(getter, None)?,
            None => Value::Undefined,
        };
        let setter = match prop.setter {
            Some(setter) => self.new_object_class(setter, None)?,
            None => Value::Undefined,
        };

        self.define_property(obj, &atom, &prop.value, &getter, &setter, prop.flags)
    }
}
