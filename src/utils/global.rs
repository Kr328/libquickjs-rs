use std::{
    ptr::NonNull,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

struct Shared {
    runtime: NonNull<rquickjs_sys::JSRuntime>,
    dirty: AtomicBool,
}

unsafe impl Send for Shared {}
unsafe impl Sync for Shared {}

struct Ref {
    shared: Arc<Shared>,
}

impl Drop for Ref {
    fn drop(&mut self) {
        self.shared.dirty.store(true, Ordering::Relaxed);
    }
}

struct Record<V> {
    value: Option<V>,
    refs: Weak<Ref>,
}

unsafe impl<V> Send for Record<V> {}
unsafe impl<V> Sync for Record<V> {}

pub struct GlobalHolder<V> {
    free: fn(NonNull<rquickjs_sys::JSRuntime>, value: V),
    shared: Arc<Shared>,
    records: Vec<Record<V>>,
}

unsafe impl<V> Send for GlobalHolder<V> {}
unsafe impl<V> Sync for GlobalHolder<V> {}

impl<V> Drop for GlobalHolder<V> {
    fn drop(&mut self) {
        for record in std::mem::take(&mut self.records) {
            if let Some(v) = record.value {
                (self.free)(self.shared.runtime, v);
            }
        }
    }
}

impl<V> GlobalHolder<V> {
    pub fn new(rt: NonNull<rquickjs_sys::JSRuntime>, free: fn(NonNull<rquickjs_sys::JSRuntime>, value: V)) -> Self {
        Self {
            free,
            shared: Arc::new(Shared {
                runtime: rt,
                dirty: AtomicBool::new(false),
            }),
            records: Vec::new(),
        }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn cleanup(&mut self) {
        if self.shared.dirty.swap(false, Ordering::Relaxed) {
            self.records.retain_mut(|v| {
                if v.refs.strong_count() > 0 {
                    true
                } else {
                    if let Some(v) = v.value.take() {
                        (self.free)(self.shared.runtime, v);
                    }

                    false
                }
            })
        }
    }

    pub fn push(&mut self, value: V) -> Global<V>
    where
        V: Clone,
    {
        let reference = Arc::new(Ref {
            shared: self.shared.clone(),
        });

        let record = Record {
            value: Some(value.clone()),
            refs: Arc::downgrade(&reference),
        };

        self.records.push(record);

        Global { reference, value }
    }
}

#[derive(Clone)]
pub struct Global<T> {
    reference: Arc<Ref>,
    value: T,
}

unsafe impl<T> Send for Global<T> {}
unsafe impl<T> Sync for Global<T> {}

impl<T: Clone> Global<T> {
    pub fn get(&self, rt: NonNull<rquickjs_sys::JSRuntime>) -> Option<T> {
        if self.reference.shared.runtime == rt {
            Some(self.value.clone())
        } else {
            None
        }
    }
}
