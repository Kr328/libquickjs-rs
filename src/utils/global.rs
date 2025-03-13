use std::{
    ptr::NonNull,
    sync::{
        Arc, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

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

        let record_weak = Arc::downgrade(&record);

        self.records.push(record);

        Global { record: record_weak }
    }
}

pub struct Global<T> {
    record: Weak<GlobalRecord<T>>,
}

unsafe impl<T> Send for Global<T> {}

impl<T> Drop for Global<T> {
    fn drop(&mut self) {
        if let Some(r) = self.record.upgrade() {
            r.detached.store(true, Ordering::Relaxed);
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
