use std::sync::{Arc, atomic::AtomicBool};

use libquickjs::{Class, Runtime};

#[test]
fn test_weak_ref() {
    let rt = Runtime::new();
    let ctx = rt.new_context();

    let closed = Arc::new(AtomicBool::new(false));
    let (tx, rx) = std::sync::mpsc::channel::<()>();

    let (promise, (resolve, reject)) = ctx.new_promise_capability().unwrap();
    {
        let weak_ref_class = ctx.get_property_str(&ctx.get_global_object(), "WeakRef").unwrap();

        let resolve = ctx.call_constructor(&weak_ref_class, None, &[resolve]).unwrap();
        let reject = ctx.call_constructor(&weak_ref_class, None, &[reject]).unwrap();
        let resolve = rt.new_global_value(&resolve).unwrap();
        let reject = rt.new_global_value(&reject).unwrap();

        let closed = closed.clone();
        std::thread::spawn(move || {
            let _ = rx.recv();

            closed.store(true, std::sync::atomic::Ordering::Relaxed);

            drop(resolve);
            drop(reject);
        });
    }

    struct SenderHolder {
        _sender: std::sync::mpsc::Sender<()>,
    }

    impl Class for SenderHolder {
        const NAME: &'static str = "SenderHolder";
    }

    let holder = ctx.new_object_class(SenderHolder { _sender: tx }, None).unwrap();
    ctx.set_property_str(&promise, "holder", holder).unwrap();

    drop(promise);
    drop(ctx);

    std::thread::sleep(std::time::Duration::from_secs(1));

    assert_eq!(closed.load(std::sync::atomic::Ordering::Relaxed), true);
}
