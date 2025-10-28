use crate::{CallOptions, Context, NativeFunction, PropertyDescriptorFlags, Value};

#[derive(Clone)]
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
    pub no_enumerable: bool,
    pub no_configurable: bool,
    pub writable: bool,
}

impl<'rt, G, S> Default for NativeProperty<'rt, G, S>
where
    G: for<'r> Fn(&Context<'r>, &Value, &Value, &[Value], CallOptions) -> Result<Value<'r>, Value<'r>> + Send + 'static,
    S: for<'r> Fn(&Context<'r>, &Value, &Value, &[Value], CallOptions) -> Result<Value<'r>, Value<'r>> + Send + 'static,
{
    fn default() -> Self {
        Self {
            value: Value::Undefined,
            getter: None,
            setter: None,
            no_enumerable: false,
            no_configurable: false,
            writable: true,
        }
    }
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
        let mut flags = PropertyDescriptorFlags::empty();

        if !prop.no_enumerable {
            flags |= PropertyDescriptorFlags::ENUMERABLE;
        }
        if !prop.no_configurable {
            flags |= PropertyDescriptorFlags::CONFIGURABLE;
        }
        if prop.writable {
            flags |= PropertyDescriptorFlags::WRITABLE;
        }

        let getter = match prop.getter {
            Some(getter) => {
                flags |= PropertyDescriptorFlags::HAS_GET;

                self.new_object_class(getter, None)?
            }
            None => Value::Undefined,
        };

        let setter = match prop.setter {
            Some(setter) => {
                flags |= PropertyDescriptorFlags::HAS_SET;

                self.new_object_class(setter, None)?
            }
            None => Value::Undefined,
        };

        match &prop.value {
            Value::Undefined => {}
            _ => {
                flags |= PropertyDescriptorFlags::HAS_VALUE;
            }
        }

        self.define_property(obj, &atom, &prop.value, &getter, &setter, flags)
    }
}
