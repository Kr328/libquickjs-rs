use serde::{
    Deserialize, Deserializer,
    de::{
        DeserializeSeed, EnumAccess, Error, IntoDeserializer, MapAccess, SeqAccess, Unexpected, VariantAccess, Visitor,
        value::SeqDeserializer,
    },
};

use crate::{
    Atom, Context, GetOwnAtomFlags, OwnAtom, Value,
    serde::{
        error::{collect_path, error_to_string},
        pool::AtomPool,
    },
};

#[derive(Clone)]
pub struct ValueDeserializer<'a, 'rt> {
    parent: Option<&'a ValueDeserializer<'a, 'rt>>,
    ctx: &'a Context<'rt>,
    key: Option<&'a Atom<'rt>>,
    value: &'a Value<'rt>,
    atom_pool: &'a AtomPool<'rt>,
}

impl<'a, 'rt> ValueDeserializer<'a, 'rt> {
    fn new(ctx: &'a Context<'rt>, value: &'a Value<'rt>, atom_pool: &'a AtomPool<'rt>) -> Self {
        Self {
            parent: None,
            ctx,
            key: None,
            value,
            atom_pool,
        }
    }
}

impl<'a, 'rt> IntoDeserializer<'rt, super::Error> for ValueDeserializer<'a, 'rt> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl<'a, 'rt> ValueDeserializer<'a, 'rt> {
    fn path(&self) -> Vec<String> {
        let mut holder = Some(self);

        collect_path(
            self.ctx,
            |d| d.key,
            std::iter::from_fn(move || holder.inspect(|h| holder = h.parent)),
        )
    }

    fn fix_path(&self, mut error: super::Error) -> super::Error {
        if error.path.is_empty() {
            error.path = self.path();
        }

        error
    }

    fn new_error(&self, repr: super::ErrorRepr) -> super::Error {
        super::Error::new(self.path(), repr)
    }

    fn value_to_error(&self, value: &Value) -> super::Error {
        self.new_error(super::ErrorRepr::EvalValue(error_to_string(self.ctx, &value)))
    }

    fn deserialize_to_string<V: Visitor<'rt>>(&self, visitor: V) -> Result<V::Value, super::Error> {
        let s = match self.value {
            Value::String(_) => self.value.clone(),
            _ => self.ctx.to_string(&self.value).map_err(|err| self.value_to_error(&err))?,
        };

        match self.ctx.get_string(&s) {
            Ok(v) => visitor.visit_str(&v).map_err(|err| self.fix_path(err)),
            Err(e) => Err(self.value_to_error(&e)),
        }
    }

    fn derive_child_value<'r>(&'r self, key: &'a Atom<'rt>, value: &'r Value<'rt>) -> ValueDeserializer<'r, 'rt> {
        ValueDeserializer {
            parent: Some(self),
            ctx: self.ctx,
            key: Some(key),
            value,
            atom_pool: self.atom_pool,
        }
    }
}

impl<'a, 'rt> Deserializer<'rt> for ValueDeserializer<'a, 'rt> {
    type Error = super::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        match self.value {
            Value::BigInt(_) => match self.ctx.to_big_int64(self.value) {
                Ok(v) => visitor.visit_i64(v).map_err(|err| self.fix_path(err)),
                Err(_) => self.deserialize_to_string(visitor),
            },
            Value::Symbol(_) => self.deserialize_to_string(visitor),
            Value::String(_) => self.deserialize_to_string(visitor),
            Value::Module(_) => self.deserialize_map(visitor),
            Value::FunctionByteCode(_) => Err(self.new_error(super::ErrorRepr::SerializingFunctionCode)),
            Value::Object(_) => {
                if self.ctx.is_array(self.value) {
                    self.deserialize_seq(visitor)
                } else {
                    self.deserialize_map(visitor)
                }
            }
            Value::Int32(v) | Value::ShortBigInt(v) => visitor.visit_i32(*v).map_err(|err| self.fix_path(err)),
            Value::Bool(v) => visitor.visit_bool(*v).map_err(|err| self.fix_path(err)),
            Value::Null | Value::Undefined | Value::Uninitialized => visitor.visit_unit().map_err(|err| self.fix_path(err)),
            Value::CatchOffset(_) => Err(self.new_error(super::ErrorRepr::SerializingCatchOffset)),
            Value::Float64(f) => visitor.visit_f64(*f).map_err(|err| self.fix_path(err)),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_to_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        match self.value {
            Value::Object(_) => {
                if self.ctx.is_array_buffer(&self.value) {
                    unsafe {
                        let buf = self
                            .ctx
                            .get_array_buffer(&self.value)
                            .map_err(|err| self.value_to_error(&err))?;

                        visitor.visit_bytes(buf).map_err(|err| self.fix_path(err))
                    }
                } else {
                    Err(self.new_error(super::ErrorRepr::ExceptingArrayBuffer))
                }
            }
            _ => Err(self.new_error(super::ErrorRepr::ExceptingArrayBuffer)),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        match self.value {
            Value::Null | Value::Undefined | Value::Uninitialized => visitor.visit_none().map_err(|err| self.fix_path(err)),
            _ => visitor.visit_some(self.clone()).map_err(|err| self.fix_path(err)),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        visitor.visit_unit().map_err(|err| self.fix_path(err))
    }

    fn deserialize_unit_struct<V>(self, _: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        visitor.visit_unit().map_err(|err| self.fix_path(err))
    }

    fn deserialize_newtype_struct<V>(self, _: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        visitor.visit_newtype_struct(self.clone()).map_err(|err| self.fix_path(err))
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        if self.ctx.is_array(&self.value) {
            struct ArrayAccess<'a, 'rt> {
                array: &'a ValueDeserializer<'a, 'rt>,
                index: u32,
                length: u32,
            }

            impl<'a, 'rt> SeqAccess<'rt> for ArrayAccess<'a, 'rt> {
                type Error = super::Error;

                fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
                where
                    T: DeserializeSeed<'rt>,
                {
                    if self.index < self.length {
                        let index = self
                            .array
                            .ctx
                            .new_atom_uint32(self.index)
                            .map_err(|err| self.array.value_to_error(&err))?;
                        let elm = self
                            .array
                            .ctx
                            .get_property(self.array.value, &index)
                            .map_err(|err| self.array.value_to_error(&err))?;

                        self.index += 1;

                        let deserializer = self.array.derive_child_value(&index, &elm);
                        seed.deserialize(deserializer.clone())
                            .map(Some)
                            .map_err(|err| deserializer.fix_path(err))
                    } else {
                        Ok(None)
                    }
                }

                fn size_hint(&self) -> Option<usize> {
                    Some((self.length - self.index) as usize)
                }
            }

            visitor
                .visit_seq(ArrayAccess {
                    array: &self,
                    index: 0,
                    length: self.ctx.get_length(&self.value).map_err(|err| self.value_to_error(&err))? as _,
                })
                .map_err(|err| self.fix_path(err))
        } else {
            struct ObjectAsSeqAccess<'a, 'rt> {
                object: &'a ValueDeserializer<'a, 'rt>,
                atoms: Vec<OwnAtom<'rt>>,
            }

            impl<'a, 'rt> SeqAccess<'rt> for ObjectAsSeqAccess<'a, 'rt> {
                type Error = super::Error;

                fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
                where
                    T: DeserializeSeed<'rt>,
                {
                    if let Some(atom) = self.atoms.pop() {
                        let key_as_value = self
                            .object
                            .ctx
                            .atom_to_value(&atom.atom)
                            .map_err(|err| self.object.value_to_error(&err))?;
                        let value_as_value = self
                            .object
                            .ctx
                            .get_property(self.object.value, &atom.atom)
                            .map_err(|err| self.object.value_to_error(&err))?;

                        let deserializer = self.object.derive_child_value(&atom.atom, &key_as_value);
                        let seq_deserializer = SeqDeserializer::new(
                            [&key_as_value, &value_as_value]
                                .map(|v| deserializer.derive_child_value(&atom.atom, v))
                                .into_iter(),
                        );

                        seed.deserialize(seq_deserializer)
                            .map(Some)
                            .map_err(|err| deserializer.fix_path(err))
                    } else {
                        Ok(None)
                    }
                }

                fn size_hint(&self) -> Option<usize> {
                    Some(self.atoms.len())
                }
            }

            let mut atoms = self
                .ctx
                .get_own_property_atoms(&self.value, GetOwnAtomFlags::STRING_MASK | GetOwnAtomFlags::ENUM_ONLY)
                .map_err(|err| self.value_to_error(&err))?;
            atoms.reverse();
            visitor
                .visit_seq(ObjectAsSeqAccess { object: &self, atoms })
                .map_err(|err| self.fix_path(err))
        }
    }

    fn deserialize_tuple<V>(self, _: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(self, _: &'static str, _: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        struct ObjectAsMapAccess<'a, 'rt> {
            object: &'a ValueDeserializer<'a, 'rt>,
            atoms: Vec<OwnAtom<'rt>>,
            next_atom_for_value: Option<OwnAtom<'rt>>,
        }

        impl<'a, 'rt> MapAccess<'rt> for ObjectAsMapAccess<'a, 'rt> {
            type Error = super::Error;

            fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
            where
                K: DeserializeSeed<'rt>,
            {
                if let Some(atom) = self.atoms.pop() {
                    let key_as_value = self
                        .object
                        .ctx
                        .atom_to_value(&atom.atom)
                        .map_err(|err| self.object.value_to_error(&err))?;
                    let deserializer = self.object.derive_child_value(&atom.atom, &key_as_value);

                    let ret = seed
                        .deserialize(deserializer.clone())
                        .map(Some)
                        .map_err(|err| deserializer.fix_path(err));

                    self.next_atom_for_value = Some(atom);

                    ret
                } else {
                    Ok(None)
                }
            }

            fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
            where
                V: DeserializeSeed<'rt>,
            {
                let key = self.next_atom_for_value.take().expect("call next value before next key");

                let value_as_value = self
                    .object
                    .ctx
                    .get_property(self.object.value, &key.atom)
                    .map_err(|err| self.object.value_to_error(&err))?;
                let deserializer = self.object.derive_child_value(&key.atom, &value_as_value);

                seed.deserialize(deserializer.clone())
                    .map_err(|err| deserializer.fix_path(err))
            }
        }

        let mut atoms = self
            .ctx
            .get_own_property_atoms(&self.value, GetOwnAtomFlags::STRING_MASK | GetOwnAtomFlags::ENUM_ONLY)
            .map_err(|err| self.value_to_error(&err))?;
        atoms.reverse();
        visitor
            .visit_map(ObjectAsMapAccess {
                object: &self,
                atoms,
                next_atom_for_value: None,
            })
            .map_err(|err| self.fix_path(err))
    }

    fn deserialize_struct<V>(self, _: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        let values = fields
            .iter()
            .map(|field| {
                let atom = self
                    .atom_pool
                    .get_or_create(self.ctx, field)
                    .map_err(|err| self.value_to_error(&err))?;
                let value = self
                    .ctx
                    .get_property(self.value, &atom)
                    .map_err(|err| self.value_to_error(&err))?;
                Ok((atom, value))
            })
            .collect::<Result<Vec<_>, Self::Error>>()?;

        visitor
            .visit_seq(SeqDeserializer::new(
                values.iter().map(|(atom, value)| self.derive_child_value(atom, value)),
            ))
            .map_err(|err| self.fix_path(err))
    }

    fn deserialize_enum<V>(self, _: &'static str, _: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        if matches!(self.value, Value::Object(_)) {
            struct ObjectAsEnumAccess<'a, 'rt> {
                object: &'a ValueDeserializer<'a, 'rt>,
            }

            impl<'a, 'rt> VariantAccess<'rt> for ObjectAsEnumAccess<'a, 'rt> {
                type Error = super::Error;

                fn unit_variant(self) -> Result<(), Self::Error> {
                    Ok(())
                }

                fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
                where
                    T: DeserializeSeed<'rt>,
                {
                    seed.deserialize(self.object.clone())
                }

                fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'rt>,
                {
                    self.object.clone().deserialize_tuple(len, visitor)
                }

                fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'rt>,
                {
                    self.object.clone().deserialize_struct("", fields, visitor)
                }
            }

            impl<'a, 'rt> EnumAccess<'rt> for ObjectAsEnumAccess<'a, 'rt> {
                type Error = super::Error;
                type Variant = Self;

                fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
                where
                    V: DeserializeSeed<'rt>,
                {
                    let constructor_atom = self
                        .object
                        .atom_pool
                        .get_or_create(self.object.ctx, "constructor")
                        .map_err(|err| self.object.value_to_error(&err))?;
                    let name_atom = self
                        .object
                        .atom_pool
                        .get_or_create(self.object.ctx, "name")
                        .map_err(|err| self.object.value_to_error(&err))?;
                    let constructor = self
                        .object
                        .ctx
                        .get_property(self.object.value, &constructor_atom)
                        .map_err(|err| self.object.value_to_error(&err))?;
                    let name = self
                        .object
                        .ctx
                        .get_property(&constructor, &name_atom)
                        .map_err(|err| self.object.value_to_error(&err))?;

                    let deserializer = self.object.derive_child_value(&constructor_atom, &name);
                    let variant_name = seed
                        .deserialize(deserializer.clone())
                        .map_err(|err| deserializer.fix_path(err))?;
                    Ok((variant_name, self))
                }
            }

            visitor
                .visit_enum(ObjectAsEnumAccess { object: &self })
                .map_err(|err| self.fix_path(err))
        } else {
            struct ValueAsEnumAccess<'a, 'rt> {
                value: &'a ValueDeserializer<'a, 'rt>,
            }

            impl<'a, 'rt> VariantAccess<'rt> for ValueAsEnumAccess<'a, 'rt> {
                type Error = super::Error;

                fn unit_variant(self) -> Result<(), Self::Error> {
                    Ok(())
                }

                fn newtype_variant_seed<T>(self, _: T) -> Result<T::Value, Self::Error>
                where
                    T: DeserializeSeed<'rt>,
                {
                    Err(Error::invalid_type(Unexpected::NewtypeVariant, &"unexpected newtype variant"))
                }

                fn tuple_variant<V>(self, _: usize, _: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'rt>,
                {
                    Err(Error::invalid_type(Unexpected::TupleVariant, &"unexpected tuple variant"))
                }

                fn struct_variant<V>(self, _: &'static [&'static str], _: V) -> Result<V::Value, Self::Error>
                where
                    V: Visitor<'rt>,
                {
                    Err(Error::invalid_type(Unexpected::StructVariant, &"unexpected struct variant"))
                }
            }

            impl<'a, 'rt> EnumAccess<'rt> for ValueAsEnumAccess<'a, 'rt> {
                type Error = super::Error;
                type Variant = Self;

                fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
                where
                    V: DeserializeSeed<'rt>,
                {
                    let variant_name = seed.deserialize(self.value.clone()).map_err(|err| self.value.fix_path(err))?;
                    Ok((variant_name, self))
                }
            }

            visitor
                .visit_enum(ValueAsEnumAccess { value: &self })
                .map_err(|err| self.fix_path(err))
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'rt>,
    {
        visitor.visit_unit().map_err(|err| self.fix_path(err))
    }

    fn is_human_readable(&self) -> bool {
        true
    }
}

pub fn from_value<'rt, D: Deserialize<'rt>>(ctx: &Context<'rt>, value: &Value<'rt>) -> Result<D, super::Error> {
    let pool = AtomPool::new();
    let deserializer = ValueDeserializer::new(ctx, value, &pool);
    D::deserialize(deserializer)
}

pub fn from_values<'rt, D: Deserialize<'rt>>(ctx: &Context<'rt>, values: &[Value<'rt>]) -> Result<Vec<D>, super::Error> {
    let pool = AtomPool::new();
    let ret = values
        .iter()
        .map(|value| {
            let deserializer = ValueDeserializer::new(ctx, value, &pool);
            D::deserialize(deserializer)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ret)
}
