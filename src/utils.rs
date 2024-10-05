use std::{
    error::Error,
    ffi::{CStr, CString},
    fmt::{Debug, Display, Formatter},
    mem::MaybeUninit,
    ops::Deref,
    ptr::NonNull,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub fn enforce_not_out_of_memory<T>(ptr: *mut T) -> NonNull<T> {
    match NonNull::new(ptr) {
        None => panic!("out of memory"),
        Some(v) => v,
    }
}

pub struct ContainNul(pub usize);

impl Debug for ContainNul {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for ContainNul {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("str contains nul char at {}", self.0))
    }
}

impl Error for ContainNul {}

pub enum MaybeTinyCString<const TINY_CAP: usize> {
    Tiny { data: [MaybeUninit<u8>; TINY_CAP], len: usize },
    Fat(CString),
}

impl<const TINY_CAP: usize> Deref for MaybeTinyCString<TINY_CAP> {
    type Target = CStr;

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeTinyCString::Tiny { data, len } => unsafe {
                CStr::from_bytes_with_nul_unchecked(std::slice::from_raw_parts(data.as_ptr() as _, *len))
            },
            MaybeTinyCString::Fat(f) => f.as_c_str(),
        }
    }
}

impl<const TINY_CAP: usize> MaybeTinyCString<TINY_CAP> {
    pub fn new(s: &[u8]) -> Result<MaybeTinyCString<TINY_CAP>, ContainNul> {
        let ret = unsafe {
            if s.len() + 1 < TINY_CAP {
                if let Some(pos) = memchr::memchr(0, s) {
                    return Err(ContainNul(pos));
                }

                let mut data = [MaybeUninit::<u8>::uninit(); TINY_CAP];

                std::slice::from_raw_parts_mut(data.as_mut_ptr() as *mut u8, s.len()).copy_from_slice(s);

                data[s.len()] = MaybeUninit::new(0);

                Self::Tiny { data, len: s.len() + 1 }
            } else {
                let s = CString::new(s).map_err(|err| ContainNul(err.nul_position()))?;

                Self::Fat(s)
            }
        };

        Ok(ret)
    }
}

pub enum MaybeTinyVec<T, const TINY_CAP: usize> {
    Tiny { data: [MaybeUninit<T>; TINY_CAP], len: usize },
    Fat(Vec<T>),
}

impl<T, const TINY_CAP: usize> Deref for MaybeTinyVec<T, TINY_CAP> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeTinyVec::Tiny { data, len } => unsafe { std::slice::from_raw_parts(data.as_ptr() as *const T, *len) },
            MaybeTinyVec::Fat(vec) => &vec[..],
        }
    }
}

impl<T, const TINY_CAP: usize> Drop for MaybeTinyVec<T, TINY_CAP> {
    fn drop(&mut self) {
        match self {
            MaybeTinyVec::Tiny { data, len } => {
                for idx in 0..*len {
                    unsafe { data[idx].assume_init_drop() }
                }
            }
            MaybeTinyVec::Fat(_) => {}
        }
    }
}

impl<T, const TINY_CAP: usize> MaybeTinyVec<T, TINY_CAP> {
    pub fn new() -> Self {
        Self::Tiny {
            data: [0; TINY_CAP].map(|_| MaybeUninit::uninit()),
            len: 0,
        }
    }

    pub fn push(&mut self, v: T) {
        match self {
            Self::Tiny { data, len } => {
                if *len + 1 > TINY_CAP {
                    let mut vec = Vec::with_capacity(*len + 1);
                    for v in std::mem::replace(data, [0; TINY_CAP].map(|_| MaybeUninit::uninit())) {
                        let v = unsafe { v.assume_init() };

                        vec.push(v);
                    }

                    vec.push(v);
                    *len = 0;

                    *self = Self::Fat(vec);

                    return;
                }

                data[*len] = MaybeUninit::new(v);
                *len += 1;
            }
            Self::Fat(fat) => {
                fat.push(v);
            }
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Tiny { len, .. } => *len,
            Self::Fat(fat) => fat.len(),
        }
    }
}

pub struct GlobalRecord<V> {
    runtime: NonNull<rquickjs_sys::JSRuntime>,
    detached: AtomicBool,
    value: V,
}

pub struct GlobalHolder<V> {
    records: Vec<Arc<GlobalRecord<V>>>,
    free: fn(NonNull<rquickjs_sys::JSRuntime>, &V),
}

impl<V> Drop for GlobalHolder<V> {
    fn drop(&mut self) {
        for record in std::mem::take(&mut self.records) {
            (self.free)(record.runtime, &record.value);
        }
    }
}

impl<V> GlobalHolder<V> {
    pub fn new(free: fn(NonNull<rquickjs_sys::JSRuntime>, &V)) -> Self {
        Self {
            records: Vec::new(),
            free,
        }
    }

    fn cleanup(&mut self) {
        self.records.retain(|v| {
            if v.detached.load(Ordering::Relaxed) {
                (self.free)(v.runtime, &v.value);

                false
            } else {
                true
            }
        })
    }

    pub fn new_global(&mut self, runtime: NonNull<rquickjs_sys::JSRuntime>, value: V) -> Global<V> {
        self.cleanup();

        let record = Arc::new(GlobalRecord {
            runtime,
            detached: AtomicBool::new(false),
            value,
        });

        self.records.push(record.clone());

        Global { runtime, record }
    }
}

pub struct Global<T> {
    runtime: NonNull<rquickjs_sys::JSRuntime>,
    record: Arc<GlobalRecord<T>>,
}

impl<T> Drop for Global<T> {
    fn drop(&mut self) {
        self.record.detached.store(true, Ordering::Relaxed);
    }
}

impl<T> Global<T> {
    pub fn as_raw(&self) -> &T {
        &self.record.value
    }

    pub fn to_local(&self, rt: NonNull<rquickjs_sys::JSRuntime>) -> Option<&T> {
        if self.runtime == rt {
            Some(&self.record.value)
        } else {
            None
        }
    }
}
