use std::sync::{Arc, atomic::AtomicUsize};

use libquickjs::Runtime;

#[test]
fn test_jobs() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    let call_count = Arc::new(AtomicUsize::new(0));

    for _ in 0..100 {
        let call_count = call_count.clone();
        ctx.enqueue_job(move || {
            call_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        })
        .unwrap();
    }

    rt.execute_pending_jobs();

    assert_eq!(call_count.load(std::sync::atomic::Ordering::Relaxed), 100);
}
