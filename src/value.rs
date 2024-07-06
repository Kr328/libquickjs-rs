use std::ffi::c_void;

use rquickjs_sys::{
    JSValue, JSValueUnion, JS_NewFloat64, JS_EXCEPTION, JS_MKPTR, JS_MKVAL, JS_NULL, JS_TAG_BIG_DECIMAL, JS_TAG_BIG_FLOAT,
    JS_TAG_BIG_INT, JS_TAG_BOOL, JS_TAG_CATCH_OFFSET, JS_TAG_EXCEPTION, JS_TAG_FLOAT64, JS_TAG_FUNCTION_BYTECODE, JS_TAG_INT,
    JS_TAG_MODULE, JS_TAG_NULL, JS_TAG_OBJECT, JS_TAG_STRING, JS_TAG_SYMBOL, JS_TAG_UNDEFINED, JS_TAG_UNINITIALIZED,
    JS_UNDEFINED, JS_UNINITIALIZED,
};

use crate::Context;

pub struct RefValue<'ctx, const TAG: i32> {
    ctx: &'ctx Context<'ctx>,
    owned: bool,
    ptr: *mut c_void,
}

impl<'ctx, const TAG: i32> Drop for RefValue<'ctx, TAG> {
    fn drop(&mut self) {
        if self.owned {
            unsafe { rquickjs_sys::JS_FreeValue(self.ctx.as_raw().as_ptr(), JS_MKPTR(TAG, self.ptr)) }
        }
    }
}

impl<'ctx, const TAG: i32> Clone for RefValue<'ctx, TAG> {
    fn clone(&self) -> Self {
        Self {
            ctx: self.ctx,
            owned: true,
            ptr: unsafe {
                let new_value = rquickjs_sys::JS_DupValue(JSValue {
                    u: JSValueUnion { ptr: self.ptr },
                    tag: TAG as _,
                });

                assert_eq!(new_value.tag, TAG as i64);

                new_value.u.ptr
            },
        }
    }
}

pub type BigDecimal<'ctx> = RefValue<'ctx, JS_TAG_BIG_DECIMAL>;
pub type BigInt<'ctx> = RefValue<'ctx, JS_TAG_BIG_INT>;
pub type BigFloat<'ctx> = RefValue<'ctx, JS_TAG_BIG_FLOAT>;
pub type Symbol<'ctx> = RefValue<'ctx, JS_TAG_SYMBOL>;
pub type String<'ctx> = RefValue<'ctx, JS_TAG_STRING>;
pub type Module<'ctx> = RefValue<'ctx, JS_TAG_MODULE>;
pub type FunctionByteCode<'ctx> = RefValue<'ctx, JS_TAG_FUNCTION_BYTECODE>;
pub type Object<'ctx> = RefValue<'ctx, JS_TAG_OBJECT>;

#[derive(Clone)]
pub enum Value<'ctx> {
    BigDecimal(BigDecimal<'ctx>),
    BigInt(BigInt<'ctx>),
    BigFloat(BigFloat<'ctx>),
    Symbol(Symbol<'ctx>),
    String(String<'ctx>),
    Module(Module<'ctx>),
    FunctionByteCode(FunctionByteCode<'ctx>),
    Object(Object<'ctx>),
    Int32(i32),
    Bool(bool),
    Null,
    Undefined,
    Uninitialized,
    CatchOffset(i32),
    Exception,
    Float64(f64),
}

unsafe impl<'ctx> Send for Value<'ctx> {}

impl<'ctx> Value<'ctx> {
    pub unsafe fn from_raw(ctx: &'ctx Context<'ctx>, value: JSValue, owned: bool) -> Self {
        #[inline]
        unsafe fn ref_value<'ctx, const TAG: i32, T: 'ctx>(
            f: fn(RefValue<'ctx, TAG>) -> T,
            ctx: &'ctx Context<'ctx>,
            value: JSValue,
            owned: bool,
        ) -> T {
            f(RefValue::<TAG> {
                ctx,
                owned,
                ptr: value.u.ptr,
            })
        }

        match value.tag as i32 {
            JS_TAG_BIG_DECIMAL => ref_value(Self::BigDecimal, ctx, value, owned),
            JS_TAG_BIG_INT => ref_value(Self::BigInt, ctx, value, owned),
            JS_TAG_BIG_FLOAT => ref_value(Self::BigFloat, ctx, value, owned),
            JS_TAG_SYMBOL => ref_value(Self::Symbol, ctx, value, owned),
            JS_TAG_STRING => ref_value(Self::String, ctx, value, owned),
            JS_TAG_MODULE => ref_value(Self::Module, ctx, value, owned),
            JS_TAG_FUNCTION_BYTECODE => ref_value(Self::FunctionByteCode, ctx, value, owned),
            JS_TAG_OBJECT => ref_value(Self::Object, ctx, value, owned),
            JS_TAG_INT => Self::Int32(value.u.int32),
            JS_TAG_BOOL => Self::Bool(value.u.int32 != 0),
            JS_TAG_NULL => Self::Null,
            JS_TAG_UNDEFINED => Self::Undefined,
            JS_TAG_UNINITIALIZED => Self::Uninitialized,
            JS_TAG_CATCH_OFFSET => Self::CatchOffset(value.u.int32),
            JS_TAG_EXCEPTION => Self::Exception,
            JS_TAG_FLOAT64 => Self::Float64(value.u.float64),
            tag => panic!("unknown tag {}", tag),
        }
    }

    pub fn as_raw(&self) -> JSValue {
        match self {
            Value::BigDecimal(v) => JS_MKPTR(JS_TAG_BIG_DECIMAL, v.ptr),
            Value::BigInt(v) => JS_MKPTR(JS_TAG_BIG_INT, v.ptr),
            Value::BigFloat(v) => JS_MKPTR(JS_TAG_BIG_FLOAT, v.ptr),
            Value::Symbol(v) => JS_MKPTR(JS_TAG_SYMBOL, v.ptr),
            Value::String(v) => JS_MKPTR(JS_TAG_STRING, v.ptr),
            Value::Module(v) => JS_MKPTR(JS_TAG_MODULE, v.ptr),
            Value::FunctionByteCode(v) => JS_MKPTR(JS_TAG_FUNCTION_BYTECODE, v.ptr),
            Value::Object(v) => JS_MKPTR(JS_TAG_OBJECT, v.ptr),
            Value::Int32(v) => JS_MKVAL(JS_TAG_INT, *v),
            Value::Bool(v) => JS_MKVAL(JS_TAG_BOOL, if *v { 1 } else { 0 }),
            Value::Null => JS_NULL,
            Value::Undefined => JS_UNDEFINED,
            Value::Uninitialized => JS_UNINITIALIZED,
            Value::CatchOffset(offset) => JS_MKVAL(JS_TAG_CATCH_OFFSET, *offset),
            Value::Exception => JS_EXCEPTION,
            Value::Float64(f) => JS_NewFloat64(*f),
        }
    }
}
