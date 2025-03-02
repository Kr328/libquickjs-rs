use std::ptr::NonNull;

pub fn enforce_not_out_of_memory<T>(ptr: *mut T) -> NonNull<T> {
    match NonNull::new(ptr) {
        None => panic!("out of memory"),
        Some(v) => v,
    }
}
