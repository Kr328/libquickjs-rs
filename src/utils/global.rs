use std::{
    ptr::NonNull,
    sync::{
        Arc,
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
        if self.runtime == rt { Some(&self.record.value) } else { None }
    }
}
