use crate::{Runtime, RuntimeStore};

#[test]
fn test_global_values() {
    let rt = Runtime::new();

    let ctx = rt.new_context();
    let global_ctx = rt.new_global_context(&ctx).unwrap();

    let value = ctx.new_object(None).unwrap();
    let global_value = rt.new_global_value(&value).unwrap();

    drop(ctx);
    drop(value);

    match rt.store() {
        RuntimeStore::Running {
            class_ids,
            global_contexts,
            global_refs,
            global_atoms,
        } => {
            global_contexts.borrow_mut().cleanup();
            global_refs.borrow_mut().cleanup();
            global_atoms.borrow_mut().cleanup();

            assert_eq!(class_ids.borrow().len(), 0);
            assert_eq!(global_contexts.borrow().len(), 1);
            assert_eq!(global_refs.borrow().len(), 1);
            assert_eq!(global_atoms.borrow().len(), 0);

            drop(global_ctx);
            drop(global_value);

            global_contexts.borrow_mut().cleanup();
            global_refs.borrow_mut().cleanup();
            global_atoms.borrow_mut().cleanup();

            assert_eq!(class_ids.borrow().len(), 0);
            assert_eq!(global_contexts.borrow().len(), 0);
            assert_eq!(global_refs.borrow().len(), 0);
            assert_eq!(global_atoms.borrow().len(), 0);
        }
        RuntimeStore::Destroying { .. } => {
            panic!("unexpected destroying runtime")
        }
    }
}

#[test]
fn test_global_auto_cleanup() {
    let rt = Runtime::new();

    let ctx = rt.new_context();
    let global_ctx = rt.new_global_context(&ctx).unwrap();

    let value = ctx.new_object(None).unwrap();
    let global_value = rt.new_global_value(&value).unwrap();

    drop(ctx);
    drop(value);

    match rt.store() {
        RuntimeStore::Running {
            class_ids,
            global_contexts,
            global_refs,
            global_atoms,
        } => {
            assert_eq!(class_ids.borrow().len(), 0);
            assert_eq!(global_contexts.borrow().len(), 1);
            assert_eq!(global_refs.borrow().len(), 1);
            assert_eq!(global_atoms.borrow().len(), 0);

            drop(global_ctx);
            drop(global_value);

            let _ = rt.new_context();

            assert_eq!(class_ids.borrow().len(), 0);
            assert_eq!(global_contexts.borrow().len(), 0);
            assert_eq!(global_refs.borrow().len(), 0);
            assert_eq!(global_atoms.borrow().len(), 0);
        }
        RuntimeStore::Destroying { .. } => {
            panic!("unexpected destroying runtime")
        }
    }
}
