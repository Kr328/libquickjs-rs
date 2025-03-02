use rquickjs_sys::{JS_FreeAtomRT, JSAtom};

use crate::Runtime;

pub struct Atom<'rt> {
    rt: &'rt Runtime,
    atom: JSAtom,
}

impl<'rt> Drop for Atom<'rt> {
    fn drop(&mut self) {
        unsafe { JS_FreeAtomRT(self.rt.as_raw().as_ptr(), self.atom) }
    }
}

impl<'rt> Atom<'rt> {
    pub unsafe fn from_raw(rt: &'rt Runtime, atom: JSAtom) -> Self {
        assert_ne!(atom, rquickjs_sys::JS_ATOM_NULL, "invalid atom");

        Self { rt, atom }
    }

    pub fn as_raw(&self) -> JSAtom {
        self.atom
    }

    pub fn get_runtime(&self) -> &'rt Runtime {
        self.rt
    }
}
