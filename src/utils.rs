use std::{
    ffi::{CStr, CString},
    mem::MaybeUninit,
    ops::Deref,
    ptr::NonNull,
};

pub fn enforce_not_out_of_memory<T>(ptr: *mut T) -> NonNull<T> {
    match NonNull::new(ptr) {
        None => panic!("out of memory"),
        Some(v) => v,
    }
}

const TINY_CSTRING_MAX_SIZE: usize = 256;

pub struct TinyCString {
    data: [MaybeUninit<u8>; TINY_CSTRING_MAX_SIZE],
    len: usize,
}

pub enum MaybeTinyCString {
    Tiny(TinyCString),
    Fat(CString),
}

impl Deref for MaybeTinyCString {
    type Target = CStr;

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeTinyCString::Tiny(t) => unsafe {
                CStr::from_bytes_with_nul_unchecked(std::slice::from_raw_parts(t.data.as_ptr() as _, t.len))
            },
            MaybeTinyCString::Fat(f) => f.as_c_str(),
        }
    }
}

impl MaybeTinyCString {
    pub fn new(s: &[u8]) -> Result<MaybeTinyCString, usize> {
        let ret = unsafe {
            let mut tiny = TinyCString {
                data: [MaybeUninit::uninit(); TINY_CSTRING_MAX_SIZE],
                len: s.len() + 1,
            };

            if tiny.len < tiny.data.len() {
                if let Some(pos) = memchr::memchr(0, s) {
                    return Err(pos);
                }

                let data_ref = std::slice::from_raw_parts_mut(tiny.data.as_mut_ptr() as *mut u8, tiny.len);

                data_ref[..s.len()].copy_from_slice(s);

                data_ref[s.len()] = 0;

                Self::Tiny(tiny)
            } else {
                let s = CString::new(s).map_err(|err| err.nul_position())?;

                Self::Fat(s)
            }
        };

        Ok(ret)
    }
}
