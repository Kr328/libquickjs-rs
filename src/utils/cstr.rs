use std::{
    ffi::{CStr, CString, NulError},
    mem::MaybeUninit,
    ops::Deref,
};

pub struct TinyCString<const CAP: usize> {
    data: [MaybeUninit<u8>; CAP],
    len: usize,
}

impl<const CAP: usize> TinyCString<CAP> {
    pub fn new(s: &[u8]) -> Option<TinyCString<CAP>> {
        unsafe {
            if s.len() + 1 <= CAP {
                if s.iter().position(|&b| b == 0).is_some() {
                    return None;
                }

                let mut data = [MaybeUninit::<u8>::uninit(); CAP];

                data.as_mut_ptr().copy_from(s.as_ptr() as _, s.len());

                data[s.len()] = MaybeUninit::new(0);

                Some(Self { data, len: s.len() + 1 })
            } else {
                None
            }
        }
    }
}

impl<const CAP: usize> Deref for TinyCString<CAP> {
    type Target = CStr;

    fn deref(&self) -> &Self::Target {
        unsafe { CStr::from_bytes_with_nul_unchecked(std::slice::from_raw_parts(self.data.as_ptr() as _, self.len)) }
    }
}

pub enum MaybeTinyCString<const TINY_CAP: usize> {
    Tiny(TinyCString<TINY_CAP>),
    Fat(CString),
}

impl<const TINY_CAP: usize> Deref for MaybeTinyCString<TINY_CAP> {
    type Target = CStr;

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeTinyCString::Tiny(s) => &s,
            MaybeTinyCString::Fat(s) => &s,
        }
    }
}

impl<const TINY_CAP: usize> MaybeTinyCString<TINY_CAP> {
    pub fn new(s: &[u8]) -> Result<MaybeTinyCString<TINY_CAP>, NulError> {
        if let Some(tiny) = TinyCString::new(s) {
            return Ok(MaybeTinyCString::Tiny(tiny));
        }

        Ok(Self::Fat(CString::new(s)?))
    }
}
