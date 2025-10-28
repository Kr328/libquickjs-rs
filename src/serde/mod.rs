mod de;
mod error;
mod pool;
mod ser;

use std::fmt::{Debug, Display, Formatter};

pub use self::{
    de::{from_value, from_values},
    ser::{to_value, to_values},
};

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

pub struct Error {
    path: Vec<String>,
    repr: ErrorRepr,
}

impl Error {
    pub fn new(path: Vec<String>, repr: ErrorRepr) -> Self {
        Self { path, repr }
    }

    pub fn object_path(&self) -> &[String] {
        &self.path
    }

    pub fn repr(&self) -> &ErrorRepr {
        &self.repr
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        struct PathDebug<'rt> {
            path: &'rt [String],
        }

        impl<'rt> Debug for PathDebug<'rt> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                f.debug_list().entries(self.path.iter()).finish()
            }
        }

        f.debug_struct("Error")
            .field("path", &PathDebug { path: &self.path })
            .field("repr", &self.repr)
            .finish()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut path = String::new();

        for v in self.path.iter() {
            path.push('.');
            path.push_str(v);
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

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

impl serde::de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self {
            path: Vec::new(),
            repr: ErrorRepr::Custom(msg.to_string()),
        }
    }
}

impl serde::ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::new(Vec::new(), ErrorRepr::Custom(msg.to_string()))
    }
}
