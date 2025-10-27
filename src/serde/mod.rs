mod de;
mod error;
mod pool;
mod ser;

use std::{
    fmt::{Debug, Display, Formatter},
    mem::ManuallyDrop,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub use self::{de::from_value, ser::to_value};
use crate::{Atom, GlobalValue, Value};

#[derive(Debug)]
pub enum ErrorRepr {
    Custom(String),
    EvalValue(String),
    SerializingFunctionCode,
    SerializingCatchOffset,
    ExceptingArrayBuffer,
    ExpectingObject,
    ExpectingArray,
}

pub struct Error<'rt> {
    path: Vec<Atom<'rt>>,
    repr: ErrorRepr,
}

impl<'rt> Error<'rt> {
    pub fn new(path: Vec<Atom<'rt>>, repr: ErrorRepr) -> Self {
        Self { path, repr }
    }

    pub fn object_path(&self) -> &[Atom<'rt>] {
        &self.path
    }

    pub fn repr(&self) -> &ErrorRepr {
        &self.repr
    }
}

impl<'rt> Debug for Error<'rt> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        struct PathDebug<'rt> {
            path: &'rt [Atom<'rt>],
        }

        impl<'rt> Debug for PathDebug<'rt> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.debug_list()
                    .entries(self.path.iter().map(|v| {
                        let ctx = v.get_runtime().new_context();
                        ctx.atom_to_string(v)
                            .and_then(|s| Ok(ctx.get_string(&s)?.to_string()))
                            .unwrap_or_else(|_| "<unknown>".to_string())
                    }))
                    .finish()
            }
        }

        f.debug_struct("Error")
            .field("path", &PathDebug { path: &self.path })
            .field("repr", &self.repr)
            .finish()
    }
}

impl<'rt> Display for Error<'rt> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut path = String::new();
        path.push_str(".");
        for (i, v) in self.path.iter().enumerate() {
            if i > 0 {
                path.push('.');
            }

            let ctx = v.get_runtime().new_context();
            match ctx.atom_to_string(v) {
                Ok(v) => match ctx.get_string(&v) {
                    Ok(s) => path.push_str(&s.to_string()),
                    Err(_) => path.push_str("<unknown>"),
                },
                Err(_) => path.push_str("<unknown>"),
            }
        }

        match &self.repr {
            ErrorRepr::Custom(msg) => write!(f, "parse {}: {}", path, msg),
            ErrorRepr::EvalValue(msg) => write!(f, "parse {}: eval error: {}", path, msg),
            ErrorRepr::SerializingFunctionCode => write!(f, "parse {}: serializing function code", path),
            ErrorRepr::SerializingCatchOffset => write!(f, "parse {}: serializing catch offset", path),
            ErrorRepr::ExceptingArrayBuffer => write!(f, "parse {}: excepting array buffer", path),
            ErrorRepr::ExpectingObject => write!(f, "parse {}: expecting object", path),
            ErrorRepr::ExpectingArray => write!(f, "parse {}: expecting array", path),
        }
    }
}

impl<'rt> std::error::Error for Error<'rt> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl<'rt> serde::de::Error for Error<'rt> {
    fn custom<T: Display>(msg: T) -> Self {
        Self {
            path: Vec::new(),
            repr: ErrorRepr::Custom(msg.to_string()),
        }
    }
}

impl<'rt> serde::ser::Error for Error<'rt> {
    fn custom<T: Display>(msg: T) -> Self {
        Self::new(Vec::new(), ErrorRepr::Custom(msg.to_string()))
    }
}

#[inline(never)]
fn type_id_of<T>() {
    std::hint::black_box(std::any::type_name::<T>());
}

impl<'rt> Deserialize<'rt> for GlobalValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'rt>,
    {
        if type_id_of::<D> as *const () == type_id_of::<de::ValueDeserializer<'_, 'rt>> as *const () {
            unsafe {
                assert_eq!(size_of::<de::ValueDeserializer<'_, 'rt>>(), size_of::<D>());

                let de = &*(&raw const deserializer as *const de::ValueDeserializer<'_, 'rt>);

                Ok(de.context().runtime().new_global_value(de.value()).expect("new global value"))
            }
        } else {
            Err(serde::de::Error::custom("unsupported deserializer"))
        }
    }
}

impl<'rt> Serialize for GlobalValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if type_id_of::<S>() == type_id_of::<ser::ValueSerializer<'_, 'rt>>() {
            unsafe {
                assert_eq!(size_of::<ser::ValueSerializer<'_, 'rt>>(), size_of::<S>());

                let ser = &*(&raw const serializer as *const ser::ValueSerializer<'_, 'rt>);
                let value = ManuallyDrop::new(self.to_local(ser.context().runtime()).expect("to local"));

                Ok(std::mem::transmute_copy::<Value<'rt>, S::Ok>(&value))
            }
        } else {
            Err(serde::ser::Error::custom("unsupported serializer"))
        }
    }
}
