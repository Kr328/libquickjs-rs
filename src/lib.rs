use std::{
    any::TypeId,
    cell::RefCell,
    collections::{HashMap, hash_map::Entry},
    ffi::CString,
    fmt::{Debug, Display, Formatter},
    mem::ManuallyDrop,
    ops::Deref,
    ptr::NonNull,
};

use bitflags::bitflags;
use rquickjs_sys::{
    JS_AddIntrinsicBaseObjects, JS_AddIntrinsicBigInt, JS_AddIntrinsicDate, JS_AddIntrinsicEval, JS_AddIntrinsicJSON,
    JS_AddIntrinsicMapSet, JS_AddIntrinsicPromise, JS_AddIntrinsicProxy, JS_AddIntrinsicRegExp, JS_AddIntrinsicRegExpCompiler,
    JS_AddIntrinsicTypedArrays, JS_AtomToString, JS_AtomToValue, JS_Call, JS_CallConstructor2, JS_DeleteProperty,
    JS_DetectModule, JS_DupAtom, JS_DupContext, JS_DupValueRT, JS_Eval, JS_EvalThis, JS_FreeAtomRT, JS_FreeCString,
    JS_FreeContext, JS_FreeRuntime, JS_FreeValueRT, JS_GetClassID, JS_GetException, JS_GetGlobalObject, JS_GetOpaque,
    JS_GetOwnProperty, JS_GetOwnPropertyNames, JS_GetProperty, JS_GetPropertyStr, JS_GetPropertyUint32, JS_GetPrototype,
    JS_GetRuntime, JS_GetRuntimeOpaque, JS_HasProperty, JS_Invoke, JS_IsArray, JS_IsConstructor, JS_IsError, JS_IsExtensible,
    JS_IsFunction, JS_IsPromise, JS_IsRegisteredClass, JS_MarkValue, JS_NewArray, JS_NewAtomLen, JS_NewAtomUInt32,
    JS_NewBigInt64, JS_NewBigUint64, JS_NewClass, JS_NewClassID, JS_NewContext, JS_NewContextRaw, JS_NewFloat64, JS_NewObject,
    JS_NewObjectClass, JS_NewObjectProto, JS_NewObjectProtoClass, JS_NewPromiseCapability, JS_NewRuntime, JS_NewStringLen,
    JS_PreventExtensions, JS_PromiseResult, JS_PromiseState, JS_ReadObject, JS_RunGC, JS_SetClassProto, JS_SetConstructorBit,
    JS_SetOpaque, JS_SetProperty, JS_SetPropertyInt64, JS_SetPropertyStr, JS_SetPropertyUint32, JS_SetPrototype,
    JS_SetRuntimeOpaque, JS_Throw, JS_ThrowTypeError, JS_ToBigInt64, JS_ToBool, JS_ToCStringLen2, JS_ToFloat64, JS_ToIndex,
    JS_ToInt32, JS_ToInt64Ext, JS_ToPropertyKey, JS_ToString, JS_ValueToAtom, JS_WriteObject, js_free,
};

use crate::utils::{
    cstr::MaybeTinyCString,
    global::{Global, GlobalHolder},
    ptr::enforce_not_out_of_memory,
    vec::MaybeTinyVec,
};
pub use crate::{atom::*, class::*, func::*, value::*};

mod atom;
mod class;
mod func;
mod utils;
mod value;

#[derive(Debug)]
pub struct InvalidRuntime;

impl Display for InvalidRuntime {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl std::error::Error for InvalidRuntime {}

pub struct GlobalContext {
    global: Global<NonNull<rquickjs_sys::JSContext>>,
}

unsafe impl Send for GlobalContext {}
unsafe impl Sync for GlobalContext {}

impl GlobalContext {
    pub fn to_local<'rt>(&self, rt: &'rt Runtime) -> Result<Context<'rt>, InvalidRuntime> {
        self.global
            .to_local(rt.rt_ptr)
            .map(|ctx| Context {
                rt,
                ptr: unsafe { enforce_not_out_of_memory(JS_DupContext(ctx.as_ptr())) },
            })
            .ok_or(InvalidRuntime)
    }
}

pub struct GlobalValue {
    global: Global<rquickjs_sys::JSValue>,
}

unsafe impl Send for GlobalValue {}
unsafe impl Sync for GlobalValue {}

impl GlobalValue {
    pub fn to_local<'rt>(&self, rt: &'rt Runtime) -> Result<Value<'rt>, InvalidRuntime> {
        self.global
            .to_local(rt.rt_ptr)
            .map(|value| unsafe { Value::from_raw(rt, JS_DupValueRT(rt.as_raw().as_ptr(), *value)).unwrap() })
            .ok_or(InvalidRuntime)
    }
}

pub struct GlobalAtom {
    global: Global<rquickjs_sys::JSAtom>,
}

unsafe impl Send for GlobalAtom {}
unsafe impl Sync for GlobalAtom {}

impl GlobalAtom {
    pub fn to_local<'rt>(&self, ctx: &Context<'rt>) -> Result<Atom<'rt>, InvalidRuntime> {
        self.global
            .to_local(ctx.rt.rt_ptr)
            .map(|atom| unsafe { Atom::from_raw(ctx.rt, JS_DupAtom(ctx.ptr.as_ptr(), *atom)) })
            .ok_or(InvalidRuntime)
    }
}

struct RuntimeStore {
    class_ids: RefCell<HashMap<TypeId, u32>>,
    global_contexts: RefCell<GlobalHolder<NonNull<rquickjs_sys::JSContext>>>,
    global_refs: RefCell<GlobalHolder<rquickjs_sys::JSValue>>,
    global_atoms: RefCell<GlobalHolder<rquickjs_sys::JSAtom>>,
}

pub struct Runtime {
    rt_ptr: NonNull<rquickjs_sys::JSRuntime>,
}

unsafe impl Send for Runtime {}

impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(JS_GetRuntimeOpaque(self.rt_ptr.as_ptr()) as *mut RuntimeStore);

            JS_FreeRuntime(self.rt_ptr.as_ptr());
        }
    }
}

impl Runtime {
    pub fn new() -> Self {
        let store = RuntimeStore {
            class_ids: RefCell::new(HashMap::new()),
            global_contexts: RefCell::new(GlobalHolder::new(|_, ctx| unsafe { JS_FreeContext(ctx.as_ptr()) })),
            global_refs: RefCell::new(GlobalHolder::new(|rt, value| unsafe { JS_FreeValueRT(rt.as_ptr(), *value) })),
            global_atoms: RefCell::new(GlobalHolder::new(|rt, value| unsafe { JS_FreeAtomRT(rt.as_ptr(), *value) })),
        };

        unsafe {
            let ptr = enforce_not_out_of_memory(JS_NewRuntime());

            JS_SetRuntimeOpaque(ptr.as_ptr(), Box::into_raw(Box::new(store)) as *mut std::ffi::c_void);

            Self { rt_ptr: ptr }
        }
    }

    pub fn as_raw(&self) -> NonNull<rquickjs_sys::JSRuntime> {
        self.rt_ptr
    }

    fn store(&self) -> &RuntimeStore {
        unsafe {
            let ptr = JS_GetRuntimeOpaque(self.rt_ptr.as_ptr());

            &*(ptr as *mut RuntimeStore)
        }
    }

    pub fn run_gc(&self) {
        unsafe { JS_RunGC(self.rt_ptr.as_ptr()) }
    }

    pub fn new_context(&self) -> Context {
        let ctx_ptr = unsafe { enforce_not_out_of_memory(JS_NewContext(self.rt_ptr.as_ptr())) };

        Context { rt: self, ptr: ctx_ptr }
    }

    pub fn new_plain_context(&self) -> Context {
        let ctx_ptr = unsafe { enforce_not_out_of_memory(JS_NewContextRaw(self.rt_ptr.as_ptr())) };

        Context { rt: self, ptr: ctx_ptr }
    }

    pub fn new_global_context(&self, ctx: &Context) -> Result<GlobalContext, InvalidRuntime> {
        if self.rt_ptr != ctx.rt.rt_ptr {
            Err(InvalidRuntime)
        } else {
            Ok(GlobalContext {
                global: self.store().global_contexts.borrow_mut().new_global(self.as_raw(), unsafe {
                    enforce_not_out_of_memory(JS_DupContext(ctx.ptr.as_ptr()))
                }),
            })
        }
    }

    pub fn new_global_value(&self, value: &Value) -> Result<GlobalValue, InvalidRuntime> {
        if matches!(value.get_runtime(), Some(rt) if rt.rt_ptr != self.rt_ptr) {
            Err(InvalidRuntime)
        } else {
            Ok(GlobalValue {
                global: self.store().global_refs.borrow_mut().new_global(self.as_raw(), unsafe {
                    JS_DupValueRT(self.as_raw().as_ptr(), value.as_raw())
                }),
            })
        }
    }

    fn get_or_alloc_class_id<C: Class>(&self) -> rquickjs_sys::JSClassID {
        let store = self.store();

        if let Some(v) = store.class_ids.borrow().get(&TypeId::of::<C>()) {
            return *v;
        }

        match store.class_ids.borrow_mut().entry(TypeId::of::<C>()) {
            Entry::Occupied(o) => *o.get(),
            Entry::Vacant(v) => {
                let mut id = 0;

                unsafe { *v.insert(JS_NewClassID(self.as_raw().as_ptr(), &mut id)) }
            }
        }
    }
}

pub struct Context<'rt> {
    rt: &'rt Runtime,
    ptr: NonNull<rquickjs_sys::JSContext>,
}

impl<'rt> Clone for Context<'rt> {
    fn clone(&self) -> Self {
        Self {
            rt: self.rt,
            ptr: unsafe { enforce_not_out_of_memory(JS_DupContext(self.ptr.as_ptr())) },
        }
    }
}

impl<'rt> Drop for Context<'rt> {
    fn drop(&mut self) {
        unsafe { JS_FreeContext(self.ptr.as_ptr()) }
    }
}

bitflags! {
    #[derive(Copy, Clone, Default)]
    pub struct EvalFlags: u32 {
        const STRICT = rquickjs_sys::JS_EVAL_FLAG_STRICT;
        const COMPILE_ONLY = rquickjs_sys::JS_EVAL_FLAG_COMPILE_ONLY;
        const BACKTRACE_BARRIER = rquickjs_sys::JS_EVAL_FLAG_BACKTRACE_BARRIER;
        const ASYNC = rquickjs_sys::JS_EVAL_FLAG_ASYNC;
    }
}

bitflags! {
    #[derive(Copy, Clone, Default)]
    pub struct Intrinsics: u64 {
        const BaseObjects = 1 << 0;
        const Date = 1 << 1;
        const Eval = 1 << 2;
        const RegExpCompiler = 1 << 4;
        const RegExp = 1 << 5;
        const JSON = 1 << 6;
        const Proxy = 1 << 7;
        const MapSet = 1 << 8;
        const TypedArrays = 1 << 9;
        const Promise = 1 << 10;
        const BigInt = 1 << 11;
    }
}

pub struct OwnAtom<'rt> {
    pub atom: Atom<'rt>,
    pub is_enumerable: bool,
}

bitflags! {
    #[derive(Copy, Clone, Default)]
    pub struct GetOwnAtomFlags: u32 {
        const STRING_MASK = rquickjs_sys::JS_GPN_STRING_MASK;
        const SYMBOL_MASK = rquickjs_sys::JS_GPN_SYMBOL_MASK;
        const ENUM_ONLY = rquickjs_sys::JS_GPN_ENUM_ONLY;
    }
}

bitflags! {
    #[derive(Copy, Clone, Default)]
    pub struct PropertyDescriptorFlags: u32 {
        const CONFIGURABLE = rquickjs_sys::JS_PROP_CONFIGURABLE;
        const WRITABLE = rquickjs_sys::JS_PROP_WRITABLE;
        const ENUMERABLE = rquickjs_sys::JS_PROP_ENUMERABLE;
        const LENGTH = rquickjs_sys::JS_PROP_LENGTH;
        const NORMAL = rquickjs_sys::JS_PROP_NORMAL;
        const GETSET = rquickjs_sys::JS_PROP_GETSET;

        const HAS_CONFIGURABLE = rquickjs_sys::JS_PROP_HAS_CONFIGURABLE;
        const HAS_WRITABLE = rquickjs_sys::JS_PROP_HAS_WRITABLE;
        const HAS_ENUMERABLE = rquickjs_sys::JS_PROP_HAS_ENUMERABLE;
        const HAS_GET = rquickjs_sys::JS_PROP_HAS_GET;
        const HAS_SET = rquickjs_sys::JS_PROP_HAS_SET;
        const HAS_VALUE = rquickjs_sys::JS_PROP_HAS_VALUE;
    }
}

pub struct PropertyDescriptor<'rt> {
    pub value: Value<'rt>,
    pub getter: Value<'rt>,
    pub setter: Value<'rt>,
    pub flags: PropertyDescriptorFlags,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PromiseState {
    Pending,
    Fulfilled,
    Rejected,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NotAPromise;

impl Display for NotAPromise {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl std::error::Error for NotAPromise {}

bitflags! {
    #[derive(Copy, Clone, Default)]
    pub struct WriteObjectFlags: u32 {
        const BYTECODE = rquickjs_sys::JS_WRITE_OBJ_BYTECODE;
        const BSWAP = rquickjs_sys::JS_WRITE_OBJ_BSWAP;
        const SAB = rquickjs_sys::JS_WRITE_OBJ_SAB;
        const REFERENCE = rquickjs_sys::JS_WRITE_OBJ_REFERENCE;
    }

    #[derive(Copy, Clone, Default)]
    pub struct ReadObjectFlags: u32 {
        const BYTECODE = rquickjs_sys::JS_READ_OBJ_BYTECODE;
        const SAB = rquickjs_sys::JS_READ_OBJ_SAB;
        const REFERENCE = rquickjs_sys::JS_READ_OBJ_REFERENCE;
    }
}

impl<'rt> Context<'rt> {
    pub fn get_runtime(&self) -> &'rt Runtime {
        self.rt
    }

    pub fn as_raw(&self) -> NonNull<rquickjs_sys::JSContext> {
        self.ptr
    }

    #[inline]
    fn enforce_ref_value_in_same_runtime<const TAG: i32>(&self, value: &RefValue<TAG>) {
        assert_eq!(
            value.get_runtime().rt_ptr,
            self.rt.rt_ptr,
            "supplied value not in same runtime"
        )
    }

    #[inline]
    fn enforce_value_in_same_runtime(&self, value: &Value) {
        match value.get_runtime() {
            None => {}
            Some(rt) => {
                assert_eq!(rt.rt_ptr, self.rt.rt_ptr, "supplied value not in same runtime")
            }
        }
    }

    #[inline]
    fn enforce_atom_in_same_runtime(&self, value: &Atom) {
        assert_eq!(
            value.get_runtime().rt_ptr,
            self.rt.rt_ptr,
            "supplied atom not in same runtime"
        )
    }

    fn new_c_string<const TINY_CAP: usize>(&self, s: impl AsRef<str>) -> Result<MaybeTinyCString<TINY_CAP>, Exception> {
        MaybeTinyCString::new(s.as_ref().as_bytes()).map_err(|pos| {
            let desc = MaybeTinyCString::<48>::new(format!("convert string: unexpected nul at {}", pos).as_bytes()).unwrap();

            unsafe { JS_ThrowTypeError(self.ptr.as_ptr(), (*desc).as_ptr()) };

            Exception
        })
    }

    fn catch(&self) -> Option<Value<'rt>> {
        unsafe {
            match Value::from_raw(self.rt, JS_GetException(self.ptr.as_ptr())).ok()? {
                Value::Null => None,
                Value::Undefined => None,
                Value::Uninitialized => None,
                v => Some(v),
            }
        }
    }

    #[inline]
    fn try_catch<R>(&self, f: impl FnOnce() -> Result<R, Exception>) -> Result<R, Value<'rt>> {
        match f() {
            Ok(ret) => Ok(ret),
            Err(_) => Err(self.catch().expect("unexpected return value from quickjs")),
        }
    }

    fn eval(
        &self,
        this: Option<&Value>,
        code: impl AsRef<str>,
        filename: impl AsRef<str>,
        flags: u32,
    ) -> Result<Value<'rt>, Value<'rt>> {
        self.try_catch(|| unsafe {
            let code = self.new_c_string::<256>(code)?;
            let filename = self.new_c_string::<64>(filename)?;

            let ret = if let Some(this) = this {
                JS_EvalThis(
                    self.ptr.as_ptr(),
                    this.as_raw(),
                    code.as_ptr(),
                    code.count_bytes() as _,
                    filename.as_ptr(),
                    flags as _,
                )
            } else {
                JS_Eval(
                    self.ptr.as_ptr(),
                    code.as_ptr(),
                    code.count_bytes() as _,
                    filename.as_ptr(),
                    flags as _,
                )
            };

            Value::from_raw(self.rt, ret)
        })
    }

    pub fn eval_global(
        &self,
        this: Option<&Value>,
        code: impl AsRef<str>,
        filename: impl AsRef<str>,
        flags: EvalFlags,
    ) -> Result<Value<'rt>, Value<'rt>> {
        self.eval(this, code, filename, flags.bits() | rquickjs_sys::JS_EVAL_TYPE_GLOBAL)
    }

    pub fn eval_module(
        &self,
        code: impl AsRef<str>,
        filename: impl AsRef<str>,
        flags: EvalFlags,
    ) -> Result<Value<'rt>, Value<'rt>> {
        self.eval(None, code, filename, flags.bits() | rquickjs_sys::JS_EVAL_TYPE_MODULE)
    }

    pub fn add_intrinsic(&self, intrinsics: Intrinsics) {
        unsafe {
            let intrinsic_func: &[(Intrinsics, unsafe extern "C" fn(*mut rquickjs_sys::JSContext))] = &[
                (Intrinsics::BaseObjects, JS_AddIntrinsicBaseObjects),
                (Intrinsics::Date, JS_AddIntrinsicDate),
                (Intrinsics::Eval, JS_AddIntrinsicEval),
                (Intrinsics::RegExpCompiler, JS_AddIntrinsicRegExpCompiler),
                (Intrinsics::RegExp, JS_AddIntrinsicRegExp),
                (Intrinsics::JSON, JS_AddIntrinsicJSON),
                (Intrinsics::Proxy, JS_AddIntrinsicProxy),
                (Intrinsics::MapSet, JS_AddIntrinsicMapSet),
                (Intrinsics::TypedArrays, JS_AddIntrinsicTypedArrays),
                (Intrinsics::Promise, JS_AddIntrinsicPromise),
                (Intrinsics::BigInt, JS_AddIntrinsicBigInt),
            ];

            for (intrinsic, add_fn) in intrinsic_func {
                if intrinsics.contains(*intrinsic) {
                    add_fn(self.ptr.as_ptr());
                }
            }
        }
    }

    pub fn new_float64(&self, v: f64) -> Value<'rt> {
        unsafe { Value::from_raw(self.rt, JS_NewFloat64(v)).unwrap() }
    }

    pub fn new_big_int64(&self, v: i64) -> Result<Value<'rt>, Value<'rt>> {
        self.try_catch(|| unsafe { Value::from_raw(self.rt, JS_NewBigInt64(self.ptr.as_ptr(), v)) })
    }

    pub fn new_big_uint64(&self, v: u64) -> Result<Value<'rt>, Value<'rt>> {
        self.try_catch(|| unsafe { Value::from_raw(self.rt, JS_NewBigUint64(self.ptr.as_ptr(), v)) })
    }

    pub fn to_bool(&self, v: &Value) -> Result<bool, Value<'rt>> {
        self.enforce_value_in_same_runtime(v);

        self.try_catch(|| unsafe {
            let ret = JS_ToBool(self.ptr.as_ptr(), v.as_raw());
            if ret < 0 { Err(Exception) } else { Ok(ret != 0) }
        })
    }

    pub fn to_int32(&self, v: &Value) -> Result<i32, Value<'rt>> {
        self.enforce_value_in_same_runtime(v);

        self.try_catch(|| unsafe {
            let mut ret = 0;

            if JS_ToInt32(self.ptr.as_ptr(), &mut ret, v.as_raw()) < 0 {
                Err(Exception)
            } else {
                Ok(ret)
            }
        })
    }

    pub fn to_int64(&self, v: &Value) -> Result<i64, Value<'rt>> {
        self.enforce_value_in_same_runtime(v);

        self.try_catch(|| unsafe {
            let mut ret = 0;

            if JS_ToInt64Ext(self.ptr.as_ptr(), &mut ret, v.as_raw()) < 0 {
                Err(Exception)
            } else {
                Ok(ret)
            }
        })
    }

    pub fn to_index(&self, v: &Value) -> Result<u64, Value<'rt>> {
        self.enforce_value_in_same_runtime(v);

        self.try_catch(|| unsafe {
            let mut ret = 0;

            if JS_ToIndex(self.ptr.as_ptr(), &mut ret, v.as_raw()) < 0 {
                Err(Exception)
            } else {
                Ok(ret)
            }
        })
    }

    pub fn to_float64(&self, v: &Value) -> Result<f64, Value<'rt>> {
        self.enforce_value_in_same_runtime(v);

        self.try_catch(|| unsafe {
            let mut ret = 0.0;

            if JS_ToFloat64(self.ptr.as_ptr(), &mut ret, v.as_raw()) < 0 {
                Err(Exception)
            } else {
                Ok(ret)
            }
        })
    }

    pub fn to_big_int64(&self, v: &Value) -> Result<i64, Value<'rt>> {
        self.enforce_value_in_same_runtime(v);

        self.try_catch(|| unsafe {
            let mut ret = 0;

            if JS_ToBigInt64(self.ptr.as_ptr(), &mut ret, v.as_raw()) < 0 {
                Err(Exception)
            } else {
                Ok(ret)
            }
        })
    }

    pub fn is_error(&self, value: &Value) -> bool {
        self.enforce_value_in_same_runtime(value);

        unsafe { JS_IsError(self.ptr.as_ptr(), value.as_raw()) }
    }

    pub fn is_function(&self, value: &Value) -> bool {
        self.enforce_value_in_same_runtime(value);

        unsafe { JS_IsFunction(self.ptr.as_ptr(), value.as_raw()) }
    }

    pub fn is_constructor(&self, value: &Value) -> bool {
        self.enforce_value_in_same_runtime(value);

        unsafe { JS_IsConstructor(self.ptr.as_ptr(), value.as_raw()) }
    }

    pub fn new_string(&self, s: impl AsRef<str>) -> Result<String<'rt>, Value<'rt>> {
        self.try_catch(|| unsafe {
            let s = s.as_ref();

            match Value::from_raw(self.rt, JS_NewStringLen(self.ptr.as_ptr(), s.as_ptr() as _, s.len() as _))? {
                Value::String(s) => Ok(s),
                value => panic!("unexpected string: {:?}", value),
            }
        })
    }

    pub fn get_string<'v>(&'v self, v: &'v String<'rt>) -> Result<JSStr<'v>, Value<'rt>>
    where
        'rt: 'v,
    {
        self.enforce_ref_value_in_same_runtime(v);

        unsafe {
            let mut length: std::ffi::c_ulong = 0;

            let ptr = JS_ToCStringLen2(self.ptr.as_ptr(), &mut length, v.as_raw(), false);
            if ptr.is_null() {
                return Err(self.catch().unwrap());
            }

            Ok(JSStr {
                ctx: self,
                ptr,
                len: length as _,
            })
        }
    }

    pub fn to_string(&self, value: &Value) -> Result<String<'rt>, Value<'rt>> {
        self.enforce_value_in_same_runtime(value);

        self.try_catch(|| unsafe {
            match Value::from_raw(self.rt, JS_ToString(self.ptr.as_ptr(), value.as_raw()))? {
                Value::String(s) => Ok(s),
                value => panic!("unexpected string: {:?}", value),
            }
        })
    }

    pub fn to_property_key(&self, value: &Value) -> Result<Value<'rt>, Value<'rt>> {
        self.enforce_value_in_same_runtime(value);

        self.try_catch(|| unsafe { Value::from_raw(self.rt, JS_ToPropertyKey(self.ptr.as_ptr(), value.as_raw())) })
    }

    #[inline]
    fn try_new_atom(&self, f: impl FnOnce() -> rquickjs_sys::JSAtom) -> Result<Atom<'rt>, Value<'rt>> {
        unsafe {
            let atom = f();

            if atom == rquickjs_sys::JS_ATOM_NULL {
                Err(self.catch().unwrap())
            } else {
                Ok(Atom::from_raw(self.rt, atom))
            }
        }
    }

    pub fn new_atom(&self, s: impl AsRef<str>) -> Result<Atom<'rt>, Value<'rt>> {
        self.try_new_atom(|| unsafe {
            let s = s.as_ref();

            JS_NewAtomLen(self.ptr.as_ptr(), s.as_ptr() as _, s.len() as _)
        })
    }

    pub fn new_atom_uint32(&self, v: u32) -> Result<Atom<'rt>, Value<'rt>> {
        self.try_new_atom(|| unsafe { JS_NewAtomUInt32(self.ptr.as_ptr(), v) })
    }

    pub fn dup_atom(&self, atom: &Atom) -> Atom<'rt> {
        self.enforce_atom_in_same_runtime(atom);

        unsafe { Atom::from_raw(self.rt, JS_DupAtom(self.ptr.as_ptr(), atom.as_raw())) }
    }

    pub fn atom_to_value(&self, atom: &Atom) -> Result<Value<'rt>, Value<'rt>> {
        self.enforce_atom_in_same_runtime(atom);

        self.try_catch(|| unsafe { Value::from_raw(self.rt, JS_AtomToValue(self.ptr.as_ptr(), atom.as_raw())) })
    }

    pub fn atom_to_string(&self, atom: &Atom) -> Result<String<'rt>, Value<'rt>> {
        self.enforce_atom_in_same_runtime(atom);

        self.try_catch(|| unsafe {
            match Value::from_raw(self.rt, JS_AtomToString(self.ptr.as_ptr(), atom.as_raw()))? {
                Value::String(v) => Ok(v),
                value => panic!("unexpected value: {:?}", value),
            }
        })
    }

    pub fn new_global_atom(&self, atom: &Atom) -> GlobalAtom {
        self.enforce_atom_in_same_runtime(atom);

        let global = self
            .rt
            .store()
            .global_atoms
            .borrow_mut()
            .new_global(self.rt.rt_ptr, atom.as_raw());

        GlobalAtom { global }
    }

    pub fn value_to_atom(&self, value: &Value) -> Result<Atom<'rt>, Value<'rt>> {
        self.enforce_value_in_same_runtime(value);

        self.try_new_atom(|| unsafe { JS_ValueToAtom(self.ptr.as_ptr(), value.as_raw()) })
    }

    fn get_or_register_class<C: Class>(&self) -> rquickjs_sys::JSClassID {
        let class_id = self.rt.get_or_alloc_class_id::<C>();

        unsafe {
            if JS_IsRegisteredClass(self.rt.as_raw().as_ptr(), class_id) {
                let name = CString::new(C::NAME).expect("invalid function name");

                let def = rquickjs_sys::JSClassDef {
                    class_name: name.as_ptr(),
                    finalizer: {
                        unsafe extern "C" fn finalizer<C: Class>(rt: *mut rquickjs_sys::JSRuntime, val: rquickjs_sys::JSValue) {
                            unsafe {
                                let rt = ManuallyDrop::new(Runtime {
                                    rt_ptr: NonNull::new(rt).unwrap(),
                                });

                                let ptr = JS_GetOpaque(val, rt.get_or_alloc_class_id::<C>());
                                if !ptr.is_null() {
                                    drop(Box::from_raw(ptr as *mut C))
                                }
                                JS_SetOpaque(val, std::ptr::null_mut());
                            }
                        }

                        Some(finalizer::<C>)
                    },
                    gc_mark: {
                        unsafe extern "C" fn gc_mark<C: Class>(
                            rt: *mut rquickjs_sys::JSRuntime,
                            val: rquickjs_sys::JSValue,
                            mark_func: rquickjs_sys::JS_MarkFunc,
                        ) {
                            struct Marker {
                                rt: NonNull<rquickjs_sys::JSRuntime>,
                                mark_func: rquickjs_sys::JS_MarkFunc,
                            }

                            impl GCMarker for Marker {
                                fn mark_value(&self, value: &Value) {
                                    unsafe { JS_MarkValue(self.rt.as_ptr(), value.as_raw(), self.mark_func) }
                                }

                                fn mark_global_value(&self, value: &GlobalValue) {
                                    unsafe { JS_MarkValue(self.rt.as_ptr(), *value.global.as_raw(), self.mark_func) }
                                }
                            }

                            let rt = ManuallyDrop::new(Runtime {
                                rt_ptr: NonNull::new(rt).unwrap(),
                            });

                            unsafe {
                                let ptr = JS_GetOpaque(val, rt.get_or_alloc_class_id::<C>()) as *const C;
                                if !ptr.is_null() {
                                    (*ptr).gc_mark(&Marker {
                                        rt: rt.as_raw(),
                                        mark_func,
                                    })
                                }
                            }
                        }

                        Some(gc_mark::<C>)
                    },
                    call: {
                        unsafe extern "C" fn call<C: Class>(
                            ctx: *mut rquickjs_sys::JSContext,
                            func_obj: rquickjs_sys::JSValue,
                            this_val: rquickjs_sys::JSValue,
                            argc: std::ffi::c_int,
                            argv: *mut rquickjs_sys::JSValue,
                            flags: std::ffi::c_int,
                        ) -> rquickjs_sys::JSValue {
                            unsafe {
                                let rt = ManuallyDrop::new(Runtime {
                                    rt_ptr: NonNull::new(JS_GetRuntime(ctx)).unwrap(),
                                });
                                let ctx = ManuallyDrop::new(Context {
                                    rt: &rt,
                                    ptr: NonNull::new(ctx).unwrap(),
                                });

                                let data = JS_GetOpaque(func_obj, JS_GetClassID(func_obj)) as *mut C;
                                if data.is_null() {
                                    panic!("unexpected function obj");
                                }

                                let func = match Value::from_raw(&rt, func_obj).unwrap() {
                                    Value::Object(obj) => ManuallyDrop::new(obj),
                                    v => panic!("invalid function object: {:?}", v),
                                };
                                let this = ManuallyDrop::new(Value::from_raw(&rt, this_val).unwrap());
                                let args = (0..argc)
                                    .into_iter()
                                    .map(|v| ManuallyDrop::new(Value::from_raw(&rt, argv.offset(v as _).read()).unwrap()))
                                    .collect::<Vec<_>>();
                                let options = CallOptions {
                                    constructor: (flags as u32) & rquickjs_sys::JS_CALL_FLAG_CONSTRUCTOR > 0,
                                };

                                let ret = match (*data).call(
                                    &ctx,
                                    &func,
                                    &this,
                                    std::slice::from_raw_parts(args.as_ptr() as _, args.len()),
                                    options,
                                ) {
                                    Ok(v) => v.into_raw(),
                                    Err(err) => JS_Throw(ctx.ptr.as_ptr(), err.into_raw()),
                                };

                                ret
                            }
                        }

                        Some(call::<C>)
                    },
                    exotic: std::ptr::null_mut(),
                };

                if JS_NewClass(self.rt.as_raw().as_ptr(), class_id, &def) != 0 {
                    panic!("out of memory")
                }

                let proto = Value::from_raw(self.rt, JS_NewObject(self.ptr.as_ptr())).unwrap();

                JS_SetClassProto(self.ptr.as_ptr(), class_id, proto.clone().into_raw());

                let proto = match proto {
                    Value::Object(obj) => obj,
                    value => panic!("unexpected value: {:?}", value),
                };

                C::on_registered(self.rt, &proto);
            }

            class_id
        }
    }

    pub fn new_object(&self, proto: Option<&Value>) -> Result<Value<'rt>, Value<'rt>> {
        if let Some(obj) = proto {
            self.enforce_value_in_same_runtime(obj);
        }

        self.try_catch(|| unsafe {
            let value = match proto {
                None => JS_NewObject(self.ptr.as_ptr()),
                Some(p) => JS_NewObjectProto(self.ptr.as_ptr(), p.as_raw()),
            };

            Value::from_raw(self.rt, value)
        })
    }

    pub fn new_object_class<C: Class>(&self, class: C, proto: Option<&Value>) -> Result<Value<'rt>, Value<'rt>> {
        if let Some(obj) = proto {
            self.enforce_value_in_same_runtime(obj);
        }

        self.try_catch(|| unsafe {
            let class_id = self.get_or_register_class::<C>();

            let value = match proto {
                None => JS_NewObjectClass(self.ptr.as_ptr(), class_id as _),
                Some(p) => JS_NewObjectProtoClass(self.ptr.as_ptr(), p.as_raw(), class_id as _),
            };

            JS_SetOpaque(value, Box::into_raw(Box::new(class)) as *mut std::ffi::c_void);

            Value::from_raw(self.rt, value)
        })
    }

    pub fn mark_as_constructor(&self, value: &Value, is_constructor: bool) -> bool {
        unsafe { JS_SetConstructorBit(self.ptr.as_ptr(), value.as_raw(), is_constructor) }
    }

    pub fn new_array(&self) -> Result<Object<'rt>, Value<'rt>> {
        self.try_catch(|| unsafe {
            match Value::from_raw(self.rt, JS_NewArray(self.ptr.as_ptr()))? {
                Value::Object(o) => Ok(o),
                value => panic!("unexpected array: {:?}", value),
            }
        })
    }

    pub fn is_array(&self, value: &Value) -> Result<bool, Value<'rt>> {
        self.enforce_value_in_same_runtime(value);

        self.try_catch(|| unsafe {
            let ret = JS_IsArray(self.ptr.as_ptr(), value.as_raw());
            if ret < 0 { Err(Exception) } else { Ok(ret != 0) }
        })
    }

    pub fn get_property(&self, obj: &Value, prop: &Atom) -> Result<Value<'rt>, Value<'rt>> {
        self.enforce_value_in_same_runtime(obj);
        self.enforce_atom_in_same_runtime(prop);

        self.try_catch(|| unsafe {
            let value = JS_GetProperty(self.ptr.as_ptr(), obj.as_raw(), prop.as_raw());
            Value::from_raw(self.rt, value)
        })
    }

    pub fn get_property_str(&self, obj: &Value, prop: impl AsRef<str>) -> Result<Value<'rt>, Value<'rt>> {
        self.enforce_value_in_same_runtime(obj);

        self.try_catch(|| unsafe {
            let prop = self.new_c_string::<64>(prop)?;

            let value = JS_GetPropertyStr(self.ptr.as_ptr(), obj.as_raw(), prop.as_ptr());
            Value::from_raw(self.rt, value)
        })
    }

    pub fn get_property_uint32(&self, obj: &Value, prop: u32) -> Result<Value<'rt>, Value<'rt>> {
        self.enforce_value_in_same_runtime(obj);

        self.try_catch(|| unsafe {
            let value = JS_GetPropertyUint32(self.ptr.as_ptr(), obj.as_raw(), prop);
            Value::from_raw(self.rt, value)
        })
    }

    pub fn set_property(&self, obj: &Value, prop: &Atom, value: Value) -> Result<(), Value<'rt>> {
        self.enforce_value_in_same_runtime(obj);
        self.enforce_value_in_same_runtime(&value);

        self.try_catch(|| unsafe {
            let ret = JS_SetProperty(self.ptr.as_ptr(), obj.as_raw(), prop.as_raw(), value.into_raw());
            if ret < 0 { Err(Exception) } else { Ok(()) }
        })
    }

    pub fn set_property_str(&self, obj: &Value, prop: impl AsRef<str>, value: Value) -> Result<(), Value<'rt>> {
        self.enforce_value_in_same_runtime(obj);
        self.enforce_value_in_same_runtime(&value);

        self.try_catch(|| unsafe {
            let prop = self.new_c_string::<64>(prop)?;

            let ret = JS_SetPropertyStr(self.ptr.as_ptr(), obj.as_raw(), prop.as_ptr(), value.into_raw());
            if ret < 0 { Err(Exception) } else { Ok(()) }
        })
    }

    pub fn set_property_uint32(&self, obj: &Value, prop: u32, value: Value) -> Result<(), Value<'rt>> {
        self.enforce_value_in_same_runtime(obj);
        self.enforce_value_in_same_runtime(&value);

        self.try_catch(|| unsafe {
            let ret = JS_SetPropertyUint32(self.ptr.as_ptr(), obj.as_raw(), prop, value.into_raw());
            if ret < 0 { Err(Exception) } else { Ok(()) }
        })
    }

    pub fn set_property_int64(&self, obj: &Value, prop: i64, value: Value) -> Result<(), Value<'rt>> {
        self.enforce_value_in_same_runtime(obj);
        self.enforce_value_in_same_runtime(&value);

        self.try_catch(|| unsafe {
            let ret = JS_SetPropertyInt64(self.ptr.as_ptr(), obj.as_raw(), prop, value.into_raw());
            if ret < 0 { Err(Exception) } else { Ok(()) }
        })
    }

    pub fn has_property(&self, obj: &Value, prop: &Atom) -> Result<bool, Value<'rt>> {
        self.enforce_value_in_same_runtime(obj);
        self.enforce_atom_in_same_runtime(prop);

        self.try_catch(|| unsafe {
            let ret = JS_HasProperty(self.ptr.as_ptr(), obj.as_raw(), prop.as_raw());
            if ret < 0 { Err(Exception) } else { Ok(ret != 0) }
        })
    }

    pub fn delete_property(&self, obj: &Value, prop: &Atom) -> Result<bool, Value<'rt>> {
        self.enforce_value_in_same_runtime(obj);
        self.enforce_atom_in_same_runtime(prop);

        self.try_catch(|| unsafe {
            let ret = JS_DeleteProperty(self.ptr.as_ptr(), obj.as_raw(), prop.as_raw(), 0);
            if ret < 0 { Err(Exception) } else { Ok(ret != 0) }
        })
    }

    pub fn is_extensible(&self, obj: &Value) -> Result<bool, Value<'rt>> {
        self.enforce_value_in_same_runtime(obj);

        self.try_catch(|| unsafe {
            let ret = JS_IsExtensible(self.ptr.as_ptr(), obj.as_raw());
            if ret < 0 { Err(Exception) } else { Ok(ret != 0) }
        })
    }

    pub fn prevent_extensions(&self, obj: &Value) -> Result<bool, Value<'rt>> {
        self.enforce_value_in_same_runtime(obj);

        self.try_catch(|| unsafe {
            let ret = JS_PreventExtensions(self.ptr.as_ptr(), obj.as_raw());
            if ret < 0 { Err(Exception) } else { Ok(ret != 0) }
        })
    }

    pub fn get_prototype(&self, value: &Value) -> Result<Value<'rt>, Value<'rt>> {
        self.enforce_value_in_same_runtime(value);

        self.try_catch(|| unsafe {
            let value = JS_GetPrototype(self.ptr.as_ptr(), value.as_raw());
            Value::from_raw(self.rt, value)
        })
    }

    pub fn set_prototype(&self, obj: &Value, proto: &Value) -> Result<bool, Value<'rt>> {
        self.enforce_value_in_same_runtime(obj);
        self.enforce_value_in_same_runtime(proto);

        self.try_catch(|| unsafe {
            let ret = JS_SetPrototype(self.ptr.as_ptr(), obj.as_raw(), proto.as_raw());
            if ret < 0 { Err(Exception) } else { Ok(ret != 0) }
        })
    }

    pub fn get_own_property_atoms(&self, obj: &Value, flags: GetOwnAtomFlags) -> Result<Vec<OwnAtom<'rt>>, Value<'rt>> {
        self.enforce_value_in_same_runtime(obj);

        self.try_catch(|| unsafe {
            let mut ptr: *mut rquickjs_sys::JSPropertyEnum = std::ptr::null_mut();
            let mut length = 0;

            let ret = JS_GetOwnPropertyNames(self.ptr.as_ptr(), &mut ptr, &mut length, obj.as_raw(), flags.bits() as _);
            if ret < 0 {
                Err(Exception)
            } else {
                let mut atoms = Vec::with_capacity(length as usize);
                for i in 0..length {
                    let current = &(*ptr.offset(i as isize));
                    atoms.push(OwnAtom {
                        atom: Atom::from_raw(self.rt, current.atom),
                        is_enumerable: current.is_enumerable,
                    });
                }
                js_free(self.ptr.as_ptr(), ptr as _);
                Ok(atoms)
            }
        })
    }

    pub fn get_own_property(&self, obj: &Value, prop: &Atom) -> Result<PropertyDescriptor<'rt>, Value<'rt>> {
        self.enforce_value_in_same_runtime(obj);
        self.enforce_atom_in_same_runtime(prop);

        self.try_catch(|| unsafe {
            let mut desc = std::mem::zeroed::<rquickjs_sys::JSPropertyDescriptor>();
            if JS_GetOwnProperty(self.ptr.as_ptr(), &mut desc, obj.as_raw(), prop.as_raw()) < 0 {
                Err(Exception)
            } else {
                Ok(PropertyDescriptor {
                    value: Value::from_raw(self.rt, desc.value).unwrap(),
                    getter: Value::from_raw(self.rt, desc.getter).unwrap(),
                    setter: Value::from_raw(self.rt, desc.setter).unwrap(),
                    flags: PropertyDescriptorFlags::from_bits_retain(desc.flags as _),
                })
            }
        })
    }

    fn convert_value_to_raw_value<const TINY_CAP: usize>(&self, args: &[Value]) -> MaybeTinyVec<rquickjs_sys::JSValue, TINY_CAP> {
        let mut vec = MaybeTinyVec::new();
        for arg in args {
            self.enforce_value_in_same_runtime(arg);
            vec.push(arg.as_raw());
        }
        vec
    }

    pub fn call(&self, func: &Value, this: &Value, args: &[Value]) -> Result<Value<'rt>, Value<'rt>> {
        self.enforce_value_in_same_runtime(func);
        self.enforce_value_in_same_runtime(this);

        let args = self.convert_value_to_raw_value::<16>(args);

        self.try_catch(|| unsafe {
            let value = JS_Call(
                self.ptr.as_ptr(),
                func.as_raw(),
                this.as_raw(),
                args.len() as _,
                args.as_ptr().cast_mut(),
            );
            Value::from_raw(self.rt, value)
        })
    }

    pub fn invoke(&self, obj: &Value, prop: &Atom, args: &[Value]) -> Result<Value<'rt>, Value<'rt>> {
        self.enforce_value_in_same_runtime(obj);
        self.enforce_atom_in_same_runtime(prop);

        let args = self.convert_value_to_raw_value::<16>(args);

        self.try_catch(|| unsafe {
            let value = JS_Invoke(
                self.ptr.as_ptr(),
                obj.as_raw(),
                prop.as_raw(),
                args.len() as _,
                args.as_ptr().cast_mut(),
            );
            Value::from_raw(self.rt, value)
        })
    }

    pub fn call_constructor(&self, func: &Value, new_target: Option<&Value>, args: &[Value]) -> Result<Value<'rt>, Value<'rt>> {
        self.enforce_value_in_same_runtime(func);

        if let Some(new_target) = new_target {
            self.enforce_value_in_same_runtime(new_target);
        }

        let args = self.convert_value_to_raw_value::<16>(args);

        self.try_catch(|| unsafe {
            let value = JS_CallConstructor2(
                self.ptr.as_ptr(),
                func.as_raw(),
                new_target.map(|v| v.as_raw()).unwrap_or(func.as_raw()),
                args.len() as _,
                args.as_ptr().cast_mut(),
            );
            Value::from_raw(self.rt, value)
        })
    }

    pub fn get_global_object(&self) -> Value<'rt> {
        unsafe { Value::from_raw(self.rt, JS_GetGlobalObject(self.ptr.as_ptr())).unwrap() }
    }

    pub fn is_promise(&self, value: &Value) -> bool {
        unsafe { JS_IsPromise(value.as_raw()) }
    }

    pub fn new_promise_capability(&self) -> Result<(Value<'rt>, (Value<'rt>, Value<'rt>)), Value<'rt>> {
        self.try_catch(|| unsafe {
            let mut resolving_funcs = [rquickjs_sys::JS_UNDEFINED, rquickjs_sys::JS_UNDEFINED];

            let ret = JS_NewPromiseCapability(self.ptr.as_ptr(), resolving_funcs.as_mut_ptr());

            let promise = Value::from_raw(self.rt, ret)?;
            let resolve = Value::from_raw(self.rt, resolving_funcs[0]).unwrap();
            let reject = Value::from_raw(self.rt, resolving_funcs[1]).unwrap();

            Ok((promise, (resolve, reject)))
        })
    }

    pub fn get_promise_state(&self, promise: &Value) -> Result<PromiseState, NotAPromise> {
        unsafe {
            let ret = JS_PromiseState(self.ptr.as_ptr(), promise.as_raw());
            match ret {
                rquickjs_sys::JSPromiseStateEnum_JS_PROMISE_PENDING => Ok(PromiseState::Pending),
                rquickjs_sys::JSPromiseStateEnum_JS_PROMISE_FULFILLED => Ok(PromiseState::Fulfilled),
                rquickjs_sys::JSPromiseStateEnum_JS_PROMISE_REJECTED => Ok(PromiseState::Rejected),
                _ => Err(NotAPromise),
            }
        }
    }

    pub fn get_promise_result(&self, promise: &Value) -> Value<'rt> {
        unsafe {
            let value = JS_PromiseResult(self.ptr.as_ptr(), promise.as_raw());
            Value::from_raw(self.rt, value).unwrap()
        }
    }

    pub fn write_object(&self, value: &Value, flags: WriteObjectFlags) -> Result<Vec<u8>, Value<'rt>> {
        unsafe {
            let mut size = 0;
            let data = JS_WriteObject(self.ptr.as_ptr(), &mut size, value.as_raw(), flags.bits() as _);
            if !data.is_null() {
                let ret = std::slice::from_raw_parts(data, size as _).to_vec();

                js_free(self.ptr.as_ptr(), data as _);

                Ok(ret)
            } else {
                Err(self.catch().expect("failed to get error in write object"))
            }
        }
    }

    pub fn read_object(&self, data: &[u8], flags: ReadObjectFlags) -> Result<Value<'rt>, Value<'rt>> {
        self.try_catch(|| unsafe {
            let value = JS_ReadObject(self.ptr.as_ptr(), data.as_ptr(), data.len() as _, flags.bits() as _);
            Value::from_raw(self.rt, value)
        })
    }
}

pub struct JSStr<'v> {
    ctx: &'v Context<'v>,
    ptr: *const std::ffi::c_char,
    len: usize,
}

impl<'v> Drop for JSStr<'v> {
    fn drop(&mut self) {
        unsafe { JS_FreeCString(self.ctx.ptr.as_ptr(), self.ptr) }
    }
}

impl<'v> Deref for JSStr<'v> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.ptr as _, self.len)) }
    }
}

pub fn detect_module(s: impl AsRef<str>) -> bool {
    match MaybeTinyCString::<128>::new(s.as_ref().as_bytes()) {
        Ok(s) => unsafe { JS_DetectModule(s.as_ptr(), s.count_bytes() as _) },
        Err(_) => false,
    }
}
