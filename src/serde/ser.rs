use serde::{
    Serializer,
    ser::{
        Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple, SerializeTupleStruct,
        SerializeTupleVariant,
    },
};

use super::{error::error_to_string, pool::AtomPool};
use crate::{Atom, Context, Value};

#[derive(Clone)]
pub struct ValueSerializer<'a, 'rt> {
    parent: Option<&'a ValueSerializer<'a, 'rt>>,
    ctx: &'a Context<'rt>,
    key: Option<&'a Atom<'rt>>,
    atom_pool: &'a AtomPool<'rt>,
}

impl<'a, 'rt> ValueSerializer<'a, 'rt> {
    pub fn new(ctx: &'a Context<'rt>, atom_pool: &'a AtomPool<'rt>) -> Self {
        Self {
            parent: None,
            ctx,
            key: None,
            atom_pool,
        }
    }

    pub fn context(&self) -> &'a Context<'rt> {
        self.ctx
    }
}

impl<'a, 'rt> ValueSerializer<'a, 'rt> {
    fn path(&self) -> Vec<Atom<'rt>> {
        let mut path = self.key.iter().map(|atom| self.ctx.dup_atom(atom)).collect::<Vec<_>>();
        let mut deserializer = self;
        while let Some(parent) = deserializer.parent {
            if let Some(key) = parent.key {
                path.push(self.ctx.dup_atom(key));
            }
            deserializer = parent;
        }
        path.reverse();
        path
    }

    fn new_error(&self, repr: super::ErrorRepr) -> super::Error<'rt> {
        super::Error::new(self.path(), repr)
    }

    fn value_to_error(&self, value: &Value) -> super::Error<'rt> {
        self.new_error(super::ErrorRepr::EvalValue(error_to_string(self.ctx, &value)))
    }

    fn derive_child_value<'r>(&'r self, key: &'a Atom<'rt>) -> ValueSerializer<'r, 'rt> {
        ValueSerializer {
            parent: Some(self),
            ctx: self.ctx,
            key: Some(key),
            atom_pool: self.atom_pool,
        }
    }
}

impl<'a, 'rt> Serializer for ValueSerializer<'a, 'rt> {
    type Ok = Value<'rt>;
    type Error = super::Error<'rt>;
    type SerializeSeq = ArrayValueSerializer<'a, 'rt>;
    type SerializeTuple = ArrayValueSerializer<'a, 'rt>;
    type SerializeTupleStruct = ArrayValueSerializer<'a, 'rt>;
    type SerializeTupleVariant = ArrayValueSerializer<'a, 'rt>;
    type SerializeMap = ObjectValueSerializer<'a, 'rt>;
    type SerializeStruct = ObjectValueSerializer<'a, 'rt>;
    type SerializeStructVariant = ObjectValueSerializer<'a, 'rt>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i32(v as i32)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i32(v as i32)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Int32(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        if let Ok(v) = i32::try_from(v) {
            Ok(Value::Int32(v))
        } else {
            self.ctx.new_big_int64(v).map_err(|err| self.value_to_error(&err))
        }
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(v as u64)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        if let Ok(v) = i32::try_from(v) {
            Ok(Value::Int32(v))
        } else {
            self.ctx.new_big_uint64(v).map_err(|err| self.value_to_error(&err))
        }
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Float64(v as f64))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Float64(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.ctx.new_string(v.to_string()).map_err(|err| self.value_to_error(&err))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.ctx.new_string(v.to_string()).map_err(|err| self.value_to_error(&err))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.ctx
            .new_array_buffer(v.to_vec(), false)
            .map_err(|err| self.value_to_error(&err))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Null)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Value::Undefined)
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
        self.ctx.new_object(None).map_err(|err| self.value_to_error(&err))
    }

    fn serialize_unit_variant(self, _: &'static str, _: u32, variant: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self, _: &'static str, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(self, _: &'static str, _: u32, _: &'static str, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(ArrayValueSerializer {
            ctx: self.ctx,
            index: 0,
            array: self.ctx.new_array().map_err(|err| self.value_to_error(&err))?,
            parent: self,
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(self, _: &'static str, len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(ObjectValueSerializer {
            ctx: self.ctx,
            atom_pool: self.atom_pool,
            object: self.ctx.new_object(None).map_err(|err| self.value_to_error(&err))?,
            next_key: None,
            parent: self,
        })
    }

    fn serialize_struct(self, _: &'static str, len: usize) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.serialize_map(Some(len))
    }
}

pub struct ArrayValueSerializer<'a, 'rt> {
    parent: ValueSerializer<'a, 'rt>,
    ctx: &'a Context<'rt>,
    index: u32,
    array: Value<'rt>,
}

impl<'a, 'rt> SerializeSeq for ArrayValueSerializer<'a, 'rt> {
    type Ok = Value<'rt>;
    type Error = super::Error<'rt>;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let atom = self
            .ctx
            .new_atom_uint32(self.index)
            .map_err(|err| self.parent.value_to_error(&err))?;

        let ser = self.parent.derive_child_value(&atom);
        let value = value.serialize(ser.clone())?;

        self.ctx
            .set_property(&self.array, &atom, value)
            .map_err(|err| ser.value_to_error(&err))?;
        self.index += 1;

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.array)
    }
}

impl<'a, 'rt> SerializeTuple for ArrayValueSerializer<'a, 'rt> {
    type Ok = Value<'rt>;
    type Error = super::Error<'rt>;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

impl<'a, 'rt> SerializeTupleStruct for ArrayValueSerializer<'a, 'rt> {
    type Ok = Value<'rt>;
    type Error = super::Error<'rt>;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

impl<'a, 'rt> SerializeTupleVariant for ArrayValueSerializer<'a, 'rt> {
    type Ok = Value<'rt>;
    type Error = super::Error<'rt>;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

pub struct ObjectValueSerializer<'a, 'rt> {
    parent: ValueSerializer<'a, 'rt>,
    ctx: &'a Context<'rt>,
    atom_pool: &'a AtomPool<'rt>,
    object: Value<'rt>,
    next_key: Option<Atom<'rt>>,
}

impl<'a, 'rt> SerializeMap for ObjectValueSerializer<'a, 'rt> {
    type Ok = Value<'rt>;
    type Error = super::Error<'rt>;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let key = key.serialize(self.parent.clone())?;

        self.next_key = Some(self.ctx.value_to_atom(&key).map_err(|err| self.parent.value_to_error(&err))?);

        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let key = self.next_key.take().expect("key is None");

        let ser = self.parent.derive_child_value(&key);
        let value = value.serialize(ser.clone())?;

        self.ctx
            .set_property(&self.object, &key, value)
            .map_err(|err| ser.value_to_error(&err))?;

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.object)
    }
}

impl<'a, 'rt> SerializeStruct for ObjectValueSerializer<'a, 'rt> {
    type Ok = Value<'rt>;
    type Error = super::Error<'rt>;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        let key = self
            .atom_pool
            .get_or_create(self.ctx, key)
            .map_err(|err| self.parent.value_to_error(&err))?;

        self.next_key = Some(key);

        SerializeMap::serialize_value(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeMap::end(self)
    }
}

impl<'a, 'rt> SerializeStructVariant for ObjectValueSerializer<'a, 'rt> {
    type Ok = Value<'rt>;
    type Error = super::Error<'rt>;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeStruct::end(self)
    }
}

pub fn to_value<'rt, S: Serialize>(ctx: &Context<'rt>, value: S) -> Result<Value<'rt>, super::Error<'rt>> {
    let pool = AtomPool::new();
    let serializer = ValueSerializer::new(ctx, &pool);
    value.serialize(serializer)
}
