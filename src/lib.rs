use std::{cell::RefCell, marker::PhantomData, ptr::NonNull};

use bitflags::bitflags;
use rquickjs_sys::{
    JS_AddIntrinsicBaseObjects, JS_AddIntrinsicBigDecimal, JS_AddIntrinsicBigFloat, JS_AddIntrinsicBigInt, JS_AddIntrinsicDate,
    JS_AddIntrinsicEval, JS_AddIntrinsicJSON, JS_AddIntrinsicMapSet, JS_AddIntrinsicOperators, JS_AddIntrinsicPromise,
    JS_AddIntrinsicProxy, JS_AddIntrinsicRegExp, JS_AddIntrinsicRegExpCompiler, JS_AddIntrinsicStringNormalize,
    JS_AddIntrinsicTypedArrays, JS_Eval, JS_FreeContext, JS_FreeRuntime, JS_FreeValueRT, JS_NewContext, JS_NewContextRaw,
    JS_NewRuntime, JS_ThrowTypeError,
};

use crate::{
    utils::{enforce_not_out_of_memory, MaybeTinyCString},
    value::Value,
};

pub mod error;
pub mod func;
mod utils;
pub mod value;

pub struct Runtime {
    rt_owned: bool,
    rt_ptr: NonNull<rquickjs_sys::JSRuntime>,
    contexts: RefCell<slab::Slab<NonNull<rquickjs_sys::JSContext>>>,
    globals: RefCell<slab::Slab<rquickjs_sys::JSValue>>,
}

unsafe impl Send for Runtime {}

impl Drop for Runtime {
    fn drop(&mut self) {
        for (_, ptr) in &*self.globals.borrow() {
            unsafe { JS_FreeValueRT(self.rt_ptr.as_ptr(), *ptr) }
        }

        for (_, ptr) in &*self.contexts.borrow() {
            unsafe { JS_FreeContext(ptr.as_ptr()) }
        }

        if self.rt_owned {
            unsafe { JS_FreeRuntime(self.rt_ptr.as_ptr()) }
        }
    }
}

impl Runtime {
    pub fn new() -> Self {
        let ptr = unsafe { enforce_not_out_of_memory(JS_NewRuntime()) };

        Self {
            rt_owned: true,
            rt_ptr: ptr,
            contexts: RefCell::new(slab::Slab::new()),
            globals: RefCell::new(slab::Slab::new()),
        }
    }

    pub fn as_raw(&self) -> NonNull<rquickjs_sys::JSRuntime> {
        self.rt_ptr
    }

    pub fn new_context(&self) -> Context {
        let ctx_ptr = unsafe { enforce_not_out_of_memory(JS_NewContext(self.rt_ptr.as_ptr())) };

        Context {
            _rt: PhantomData,
            ctx_owned: true,
            ctx_ptr,
        }
    }

    pub fn new_plain_context(&self) -> Context {
        let ctx_ptr = unsafe { enforce_not_out_of_memory(JS_NewContextRaw(self.rt_ptr.as_ptr())) };

        Context {
            _rt: PhantomData,
            ctx_owned: true,
            ctx_ptr,
        }
    }

    pub fn get_context(&self, id: ContextID) -> Option<Context> {
        let contexts = self.contexts.borrow();

        let r = contexts.get(id.index)?;
        if (r.as_ptr() as usize) == id.ptr {
            Some(Context {
                _rt: PhantomData,
                ctx_owned: false,
                ctx_ptr: *r,
            })
        } else {
            None
        }
    }

    pub fn take_context(&self, id: ContextID) -> Option<Context> {
        let mut contexts = self.contexts.borrow_mut();

        let ctx_ptr = contexts.get(id.index)?;
        if (ctx_ptr.as_ptr() as usize) == id.ptr {
            let ctx_ptr = contexts.remove(id.index);

            Some(Context {
                _rt: PhantomData,
                ctx_owned: true,
                ctx_ptr,
            })
        } else {
            None
        }
    }

    pub fn store_context(&self, mut context: Context) -> ContextID {
        context.ctx_owned = false;

        let index = self.contexts.borrow_mut().insert(context.ctx_ptr);

        ContextID {
            index,
            ptr: context.ctx_ptr.as_ptr() as _,
        }
    }
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct ContextID {
    index: usize,
    ptr: usize,
}

pub struct Context<'rt> {
    _rt: PhantomData<&'rt Runtime>,
    ctx_owned: bool,
    ctx_ptr: NonNull<rquickjs_sys::JSContext>,
}

impl<'rt> Drop for Context<'rt> {
    fn drop(&mut self) {
        if self.ctx_owned {
            unsafe { JS_FreeContext(self.ctx_ptr.as_ptr()) }
        }
    }
}

macro_rules! new_cstring_or_throw {
    ($var:expr, $ctx:expr) => {
        match MaybeTinyCString::new($var) {
            Ok(s) => s,
            Err(pos) => {
                let desc = MaybeTinyCString::new(format!("convert string: unexpected nul at {}", pos).as_bytes()).unwrap();

                unsafe { JS_ThrowTypeError($ctx, (*desc).as_ptr()) };

                return Value::Exception;
            }
        }
    };
}

bitflags! {
    #[derive(Copy, Clone, Default)]
    pub struct EvalFlags: u32 {
        const STRICT = rquickjs_sys::JS_EVAL_FLAG_STRICT;
        const STRIP = rquickjs_sys::JS_EVAL_FLAG_STRIP;
        const COMPILE_ONLY = rquickjs_sys::JS_EVAL_FLAG_COMPILE_ONLY;
        const BACKTRACE_BARRIER = rquickjs_sys::JS_EVAL_FLAG_BACKTRACE_BARRIER;
        const ASYNC = rquickjs_sys::JS_EVAL_FLAG_ASYNC;
    }
}

pub enum Intrinsic {
    BaseObjects,
    Date,
    Eval,
    StringNormalize,
    RegExpCompiler,
    RegExp,
    JSON,
    Proxy,
    MapSet,
    TypedArrays,
    Promise,
    BigInt,
    BigFloat,
    BigDecimal,
    Operators,
}

impl<'rt> Context<'rt> {
    pub fn as_raw(&self) -> NonNull<rquickjs_sys::JSContext> {
        self.ctx_ptr
    }

    fn eval(&self, code: impl AsRef<str>, filename: impl AsRef<str>, flags: u32) -> Value {
        let code = new_cstring_or_throw!(code.as_ref().as_bytes(), self.ctx_ptr.as_ptr());
        let filename = new_cstring_or_throw!(filename.as_ref().as_bytes(), self.ctx_ptr.as_ptr());

        unsafe {
            let ret = JS_Eval(
                self.ctx_ptr.as_ptr(),
                code.as_ptr(),
                code.count_bytes() as _,
                filename.as_ptr(),
                flags as _,
            );

            Value::from_raw(self, ret, true)
        }
    }

    pub fn eval_global(&self, code: impl AsRef<str>, filename: impl AsRef<str>, flags: EvalFlags) -> Value {
        self.eval(code, filename, flags.bits() | rquickjs_sys::JS_EVAL_TYPE_GLOBAL)
    }

    pub fn eval_module(&self, code: impl AsRef<str>, filename: impl AsRef<str>, flags: EvalFlags) -> Value {
        self.eval(code, filename, flags.bits() | rquickjs_sys::JS_EVAL_TYPE_MODULE)
    }

    pub fn add_intrinsic(&self, intrinsic: Intrinsic) {
        unsafe {
            match intrinsic {
                Intrinsic::BaseObjects => JS_AddIntrinsicBaseObjects(self.ctx_ptr.as_ptr()),
                Intrinsic::Date => JS_AddIntrinsicDate(self.ctx_ptr.as_ptr()),
                Intrinsic::Eval => JS_AddIntrinsicEval(self.ctx_ptr.as_ptr()),
                Intrinsic::StringNormalize => JS_AddIntrinsicStringNormalize(self.ctx_ptr.as_ptr()),
                Intrinsic::RegExpCompiler => JS_AddIntrinsicRegExpCompiler(self.ctx_ptr.as_ptr()),
                Intrinsic::RegExp => JS_AddIntrinsicRegExp(self.ctx_ptr.as_ptr()),
                Intrinsic::JSON => JS_AddIntrinsicJSON(self.ctx_ptr.as_ptr()),
                Intrinsic::Proxy => JS_AddIntrinsicProxy(self.ctx_ptr.as_ptr()),
                Intrinsic::MapSet => JS_AddIntrinsicMapSet(self.ctx_ptr.as_ptr()),
                Intrinsic::TypedArrays => JS_AddIntrinsicTypedArrays(self.ctx_ptr.as_ptr()),
                Intrinsic::Promise => JS_AddIntrinsicPromise(self.ctx_ptr.as_ptr()),
                Intrinsic::BigInt => JS_AddIntrinsicBigInt(self.ctx_ptr.as_ptr()),
                Intrinsic::BigFloat => JS_AddIntrinsicBigFloat(self.ctx_ptr.as_ptr()),
                Intrinsic::BigDecimal => JS_AddIntrinsicBigDecimal(self.ctx_ptr.as_ptr()),
                Intrinsic::Operators => JS_AddIntrinsicOperators(self.ctx_ptr.as_ptr()),
            }
        }
    }
}
