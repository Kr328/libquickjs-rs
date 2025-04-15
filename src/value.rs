use std::{
    ffi::c_void,
    fmt::{Debug, Formatter},
};

use rquickjs_sys::{
    JS_EXCEPTION, JS_FreeCString, JS_MKPTR, JS_MKVAL, JS_NULL, JS_NewFloat64, JS_TAG_BIG_INT, JS_TAG_BOOL, JS_TAG_CATCH_OFFSET,
    JS_TAG_EXCEPTION, JS_TAG_FLOAT64, JS_TAG_FUNCTION_BYTECODE, JS_TAG_INT, JS_TAG_MODULE, JS_TAG_NULL, JS_TAG_OBJECT,
    JS_TAG_STRING, JS_TAG_SYMBOL, JS_TAG_UNDEFINED, JS_TAG_UNINITIALIZED, JS_ToCStringLen, JS_UNDEFINED, JS_UNINITIALIZED,
    JS_VALUE_IS_NAN, JSValue, JSValueUnion,
};

use crate::Runtime;

#[derive(Copy, Clone, Debug)]
pub struct Exception;

impl Exception {
    pub fn as_raw(&self) -> JSValue {
        JS_EXCEPTION
    }
}

pub struct RefValue<'rt, const TAG: i32> {
    rt: &'rt Runtime,
    ptr: *mut c_void,
}

impl<'rt, const TAG: i32> PartialEq for RefValue<'rt, TAG> {
    fn eq(&self, other: &Self) -> bool {
        self.rt.ptr == other.rt.ptr && self.ptr == other.ptr
    }
}

impl<'rt, const TAG: i32> RefValue<'rt, TAG> {
    pub fn get_runtime(&self) -> &'rt Runtime {
        self.rt
    }
}

impl<'rt, const TAG: i32> Debug for RefValue<'rt, TAG> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let ctx = self.rt.new_context();

            let mut length = 0;
            let data = JS_ToCStringLen(ctx.as_raw().as_ptr(), &mut length, self.as_raw());
            if data.is_null() {
                f.write_fmt(format_args!("Ref(tag: {}, ptr: {:?})", TAG, self.ptr))
            } else {
                f.write_fmt(format_args!(
                    "Ref(tag: {}, ptr: {:?}, value: {})",
                    TAG,
                    self.ptr,
                    std::str::from_utf8_unchecked(std::slice::from_raw_parts(data as _, length))
                ))?;

                JS_FreeCString(ctx.as_raw().as_ptr(), data);

                Ok(())
            }
        }
    }
}

impl<'rt, const TAG: i32> Drop for RefValue<'rt, TAG> {
    fn drop(&mut self) {
        unsafe { rquickjs_sys::JS_FreeValueRT(self.rt.as_raw().as_ptr(), JS_MKPTR(TAG, self.ptr)) }
    }
}

impl<'rt, const TAG: i32> Clone for RefValue<'rt, TAG> {
    fn clone(&self) -> Self {
        Self {
            rt: self.rt,
            ptr: unsafe {
                let new_value = rquickjs_sys::JS_DupValueRT(
                    self.rt.as_raw().as_ptr(),
                    JSValue {
                        u: JSValueUnion { ptr: self.ptr },
                        tag: TAG as _,
                    },
                );

                assert_eq!(new_value.tag, TAG as i64);

                new_value.u.ptr
            },
        }
    }
}

impl<'rt, const TAG: i32> RefValue<'rt, TAG> {
    pub fn as_raw(&self) -> JSValue {
        JS_MKPTR(TAG, self.ptr)
    }
}

pub type BigInt<'rt> = RefValue<'rt, JS_TAG_BIG_INT>;
pub type Symbol<'rt> = RefValue<'rt, JS_TAG_SYMBOL>;
pub type String<'rt> = RefValue<'rt, JS_TAG_STRING>;
pub type Module<'rt> = RefValue<'rt, JS_TAG_MODULE>;
pub type FunctionByteCode<'rt> = RefValue<'rt, JS_TAG_FUNCTION_BYTECODE>;
pub type Object<'rt> = RefValue<'rt, JS_TAG_OBJECT>;

#[derive(Clone, Debug, PartialEq)]
pub enum Value<'rt> {
    BigInt(BigInt<'rt>),
    Symbol(Symbol<'rt>),
    String(String<'rt>),
    Module(Module<'rt>),
    FunctionByteCode(FunctionByteCode<'rt>),
    Object(Object<'rt>),
    Int32(i32),
    Bool(bool),
    Null,
    Undefined,
    Uninitialized,
    CatchOffset(i32),
    Float64(f64),
}

impl<'rt> Value<'rt> {
    pub fn is_nan(&self) -> bool {
        match self {
            Self::Float64(_) => unsafe { JS_VALUE_IS_NAN(self.as_raw()) },
            _ => false,
        }
    }
}

impl<'rt> From<BigInt<'rt>> for Value<'rt> {
    fn from(value: BigInt<'rt>) -> Self {
        Self::BigInt(value)
    }
}

impl<'rt> From<Symbol<'rt>> for Value<'rt> {
    fn from(value: Symbol<'rt>) -> Self {
        Self::Symbol(value)
    }
}

impl<'rt> From<String<'rt>> for Value<'rt> {
    fn from(value: String<'rt>) -> Self {
        Self::String(value)
    }
}

impl<'rt> From<Module<'rt>> for Value<'rt> {
    fn from(value: Module<'rt>) -> Self {
        Self::Module(value)
    }
}

impl<'rt> From<FunctionByteCode<'rt>> for Value<'rt> {
    fn from(value: FunctionByteCode<'rt>) -> Self {
        Self::FunctionByteCode(value)
    }
}

impl<'rt> From<Object<'rt>> for Value<'rt> {
    fn from(value: Object<'rt>) -> Self {
        Self::Object(value)
    }
}

impl<'rt> From<i32> for Value<'rt> {
    fn from(value: i32) -> Self {
        Self::Int32(value)
    }
}

impl From<bool> for Value<'_> {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<f64> for Value<'_> {
    fn from(value: f64) -> Self {
        let jsv = JS_NewFloat64(value);
        unsafe {
            match jsv.tag as i32 {
                JS_TAG_INT => Self::Int32(jsv.u.int32),
                JS_TAG_FLOAT64 => Self::Float64(jsv.u.float64),
                _ => Self::Float64(value),
            }
        }
    }
}

impl<'rt> Value<'rt> {
    pub unsafe fn from_raw(rt: &'rt Runtime, value: JSValue) -> Result<Self, Exception> {
        unsafe {
            #[inline]
            unsafe fn ref_value<'rt, const TAG: i32, T: 'rt>(
                f: fn(RefValue<'rt, TAG>) -> T,
                rt: &'rt Runtime,
                value: JSValue,
            ) -> T {
                unsafe { f(RefValue::<TAG> { rt, ptr: value.u.ptr }) }
            }

            Ok(match value.tag as i32 {
                JS_TAG_BIG_INT => ref_value(Self::BigInt, rt, value),
                JS_TAG_SYMBOL => ref_value(Self::Symbol, rt, value),
                JS_TAG_STRING => ref_value(Self::String, rt, value),
                JS_TAG_MODULE => ref_value(Self::Module, rt, value),
                JS_TAG_FUNCTION_BYTECODE => ref_value(Self::FunctionByteCode, rt, value),
                JS_TAG_OBJECT => ref_value(Self::Object, rt, value),
                JS_TAG_INT => Self::Int32(value.u.int32),
                JS_TAG_BOOL => Self::Bool(value.u.int32 != 0),
                JS_TAG_NULL => Self::Null,
                JS_TAG_UNDEFINED => Self::Undefined,
                JS_TAG_UNINITIALIZED => Self::Uninitialized,
                JS_TAG_CATCH_OFFSET => Self::CatchOffset(value.u.int32),
                JS_TAG_FLOAT64 => Self::Float64(value.u.float64),
                JS_TAG_EXCEPTION => return Err(Exception),
                tag => panic!("unknown tag {}", tag),
            })
        }
    }

    pub fn as_raw(&self) -> JSValue {
        match self {
            Value::BigInt(v) => v.as_raw(),
            Value::Symbol(v) => v.as_raw(),
            Value::String(v) => v.as_raw(),
            Value::Module(v) => v.as_raw(),
            Value::FunctionByteCode(v) => v.as_raw(),
            Value::Object(v) => v.as_raw(),
            Value::Int32(v) => JS_MKVAL(JS_TAG_INT, *v),
            Value::Bool(v) => JS_MKVAL(JS_TAG_BOOL, if *v { 1 } else { 0 }),
            Value::Null => JS_NULL,
            Value::Undefined => JS_UNDEFINED,
            Value::Uninitialized => JS_UNINITIALIZED,
            Value::CatchOffset(offset) => JS_MKVAL(JS_TAG_CATCH_OFFSET, *offset),
            Value::Float64(f) => JS_NewFloat64(*f),
        }
    }

    pub fn into_raw(self) -> JSValue {
        fn detach_ptr<const TAG: i32>(r: RefValue<TAG>) -> *mut c_void {
            let ptr = r.ptr;

            std::mem::forget(r);

            ptr
        }

        match self {
            Value::BigInt(v) => JS_MKPTR(JS_TAG_BIG_INT, v.ptr),
            Value::Symbol(v) => JS_MKPTR(JS_TAG_SYMBOL, v.ptr),
            Value::String(v) => JS_MKPTR(JS_TAG_STRING, detach_ptr(v)),
            Value::Module(v) => JS_MKPTR(JS_TAG_MODULE, detach_ptr(v)),
            Value::FunctionByteCode(v) => JS_MKPTR(JS_TAG_FUNCTION_BYTECODE, detach_ptr(v)),
            Value::Object(v) => JS_MKPTR(JS_TAG_OBJECT, detach_ptr(v)),
            Value::Int32(v) => JS_MKVAL(JS_TAG_INT, v),
            Value::Bool(v) => JS_MKVAL(JS_TAG_BOOL, if v { 1 } else { 0 }),
            Value::Null => JS_NULL,
            Value::Undefined => JS_UNDEFINED,
            Value::Uninitialized => JS_UNINITIALIZED,
            Value::CatchOffset(offset) => JS_MKVAL(JS_TAG_CATCH_OFFSET, offset),
            Value::Float64(f) => JS_NewFloat64(f),
        }
    }

    pub fn get_runtime(&self) -> Option<&'rt Runtime> {
        match self {
            Value::BigInt(v) => Some(v.get_runtime()),
            Value::Symbol(v) => Some(v.get_runtime()),
            Value::String(v) => Some(v.get_runtime()),
            Value::Module(v) => Some(v.get_runtime()),
            Value::FunctionByteCode(v) => Some(v.get_runtime()),
            Value::Object(v) => Some(v.get_runtime()),
            _ => None,
        }
    }
}

pub trait ValueResultExt {
    fn as_raw(&self) -> JSValue;
}

impl<'a> ValueResultExt for Result<Value<'a>, Exception> {
    fn as_raw(&self) -> JSValue {
        match self {
            Ok(v) => v.as_raw(),
            Err(e) => e.as_raw(),
        }
    }
}
