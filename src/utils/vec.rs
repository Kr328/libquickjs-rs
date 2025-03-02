use std::{
    mem::{ManuallyDrop, MaybeUninit},
    ops::{Deref, DerefMut},
    vec,
};

pub struct TinyVec<T, const CAP: usize> {
    data: [MaybeUninit<T>; CAP],
    len: usize,
}

impl<T, const TINY_CAP: usize> TinyVec<T, TINY_CAP> {
    pub fn new() -> Self {
        Self {
            data: [0; TINY_CAP].map(|_| MaybeUninit::uninit()),
            len: 0,
        }
    }

    pub fn try_push(&mut self, v: T) -> Result<(), T> {
        if self.len < TINY_CAP {
            self.data[self.len] = MaybeUninit::new(v);
            self.len += 1;
            Ok(())
        } else {
            Err(v)
        }
    }
}

impl<T, const CAP: usize> Deref for TinyVec<T, CAP> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.data.as_ptr() as *const T, self.len) }
    }
}

impl<T, const CAP: usize> DerefMut for TinyVec<T, CAP> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.data.as_mut_ptr() as *mut T, self.len) }
    }
}

impl<T, const CAP: usize> Drop for TinyVec<T, CAP> {
    fn drop(&mut self) {
        for idx in 0..self.len {
            unsafe { self.data[idx].assume_init_drop() }
        }
    }
}

pub struct TinyVecIntoIter<T, const CAP: usize> {
    data: [MaybeUninit<T>; CAP],
    len: usize,
    idx: usize,
}

impl<T, const CAP: usize> Iterator for TinyVecIntoIter<T, CAP> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx < self.len {
            let ret = unsafe { self.data[self.idx].assume_init_read() };
            self.idx += 1;
            Some(ret)
        } else {
            None
        }
    }
}

impl<T, const CAP: usize> Drop for TinyVecIntoIter<T, CAP> {
    fn drop(&mut self) {
        for idx in self.idx..self.len {
            unsafe { self.data[idx].assume_init_drop() }
        }
    }
}

impl<T, const CAP: usize> IntoIterator for TinyVec<T, CAP> {
    type Item = T;
    type IntoIter = TinyVecIntoIter<T, CAP>;

    fn into_iter(mut self) -> Self::IntoIter {
        TinyVecIntoIter {
            data: std::mem::replace(&mut self.data, [0; CAP].map(|_| MaybeUninit::uninit())),
            len: std::mem::replace(&mut self.len, 0),
            idx: 0,
        }
    }
}

pub enum MaybeTinyVec<T, const TINY_CAP: usize> {
    Tiny(TinyVec<T, TINY_CAP>),
    Fat(Vec<T>),
}

impl<T, const TINY_CAP: usize> FromIterator<T> for MaybeTinyVec<T, TINY_CAP> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();

        if iter.size_hint().0 > TINY_CAP {
            Self::Fat(Vec::from_iter(iter))
        } else {
            let mut ret = Self::new();

            for v in iter {
                ret.push(v);
            }

            ret
        }
    }
}

pub enum MaybeTinyVecIntoIter<T, const TINY_CAP: usize> {
    Tiny(TinyVecIntoIter<T, TINY_CAP>),
    Fat(vec::IntoIter<T>),
}

impl<T, const TINY_CAP: usize> Iterator for MaybeTinyVecIntoIter<T, TINY_CAP> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            MaybeTinyVecIntoIter::Tiny(tiny) => tiny.next(),
            MaybeTinyVecIntoIter::Fat(fat) => fat.next(),
        }
    }
}

impl<T, const TINY_CAP: usize> IntoIterator for MaybeTinyVec<T, TINY_CAP> {
    type Item = T;
    type IntoIter = MaybeTinyVecIntoIter<T, TINY_CAP>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            MaybeTinyVec::Tiny(tiny) => MaybeTinyVecIntoIter::Tiny(tiny.into_iter()),
            MaybeTinyVec::Fat(fat) => MaybeTinyVecIntoIter::Fat(fat.into_iter()),
        }
    }
}

impl<T, const TINY_CAP: usize> Deref for MaybeTinyVec<T, TINY_CAP> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeTinyVec::Tiny(tiny) => &tiny,
            MaybeTinyVec::Fat(vec) => &vec,
        }
    }
}

impl<T, const TINY_CAP: usize> DerefMut for MaybeTinyVec<T, TINY_CAP> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            MaybeTinyVec::Tiny(tiny) => tiny,
            MaybeTinyVec::Fat(vec) => vec,
        }
    }
}

impl<T, const TINY_CAP: usize> MaybeTinyVec<T, TINY_CAP> {
    pub fn new() -> Self {
        Self::Tiny(TinyVec::new())
    }

    pub fn push(&mut self, v: T) {
        match self {
            Self::Tiny(tiny) => match tiny.try_push(v) {
                Ok(_) => return,
                Err(v) => unsafe {
                    let tiny = ManuallyDrop::new(std::mem::replace(tiny, TinyVec::new()));
                    let mut fat = Vec::<T>::with_capacity(tiny.len + 1);

                    fat.as_mut_ptr().copy_from(tiny.data.as_ptr() as *const T, tiny.len);
                    fat.set_len(tiny.len);

                    fat.push(v);

                    *self = Self::Fat(fat);
                },
            },
            Self::Fat(fat) => {
                fat.push(v);
            }
        }
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Tiny(tiny) => tiny.len(),
            Self::Fat(fat) => fat.len(),
        }
    }
}
