use std::{
    cell::RefCell,
    collections::{HashMap, hash_map::Entry},
};

use crate::{Atom, Context, Value};

pub struct AtomPool<'rt> {
    atoms: RefCell<HashMap<&'static str, Atom<'rt>>>,
}

impl<'rt> AtomPool<'rt> {
    pub fn new() -> Self {
        Self {
            atoms: RefCell::new(HashMap::new()),
        }
    }

    pub fn get_or_create(&self, ctx: &Context<'rt>, name: &'static str) -> Result<Atom<'rt>, Value<'rt>> {
        match self.atoms.borrow_mut().entry(name) {
            Entry::Occupied(entry) => Ok(ctx.dup_atom(entry.get())),
            Entry::Vacant(entry) => {
                let atom = ctx.new_atom(name)?;
                Ok(ctx.dup_atom(entry.insert(atom)))
            }
        }
    }
}
