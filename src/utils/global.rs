use std::{
    ptr::NonNull,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

struct Record<V> {
    runtime: NonNull<rquickjs_sys::JSRuntime>,
    detached: AtomicBool,
    value: V,
}

pub struct GlobalHolder<V> {
    dirty: Arc<AtomicBool>,
    records: Vec<Arc<Record<V>>>,
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
            dirty: Arc::new(AtomicBool::new(false)),
            records: Vec::new(),
            free,
        }
    }

    pub fn cleanup(&mut self) {
        if self.dirty.swap(false, Ordering::Relaxed) {
            self.records.retain(|v| {
                if v.detached.load(Ordering::Relaxed) {
                    (self.free)(v.runtime, &v.value);

                    false
                } else {
                    true
                }
            })
        }
    }

    pub fn push(&mut self, runtime: NonNull<rquickjs_sys::JSRuntime>, value: V) -> Global<V> {
        let record = Arc::new(Record {
            runtime,
            detached: AtomicBool::new(false),
            value,
        });

        let record_weak = Arc::downgrade(&record);

        self.records.push(record);

        Global {
            dirty: Arc::downgrade(&self.dirty),
            record: record_weak,
        }
    }
}

#[derive(Clone)]
pub struct Global<T> {
    dirty: Weak<AtomicBool>,
    record: Weak<Record<T>>,
}

unsafe impl<T> Send for Global<T> {}
unsafe impl<T> Sync for Global<T> {}

impl<T> Drop for Global<T> {
    fn drop(&mut self) {
        if let Some(r) = self.record.upgrade() {
            r.detached.store(true, Ordering::Relaxed);
        }
        if let Some(d) = self.dirty.upgrade() {
            d.store(true, Ordering::Relaxed);
        }
    }
}

impl<T: Clone> Global<T> {
    pub fn get(&self, rt: Option<NonNull<rquickjs_sys::JSRuntime>>) -> Option<T> {
        let record = self.record.upgrade()?;

        if let Some(rt) = rt {
            if record.runtime != rt {
                return None;
            }
        }

        Some(record.value.clone())
    }
}
