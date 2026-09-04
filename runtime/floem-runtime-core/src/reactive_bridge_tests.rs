use super::{BindingRegistry, ReactiveBinding};
use crate::arena::ElementArena;
use crate::dirty::{DirtyFlags, DirtySet};
use floem_reactive::{RwSignal, SignalUpdate};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn signal_update_dirties_only_bound_element() {
    // Do not call Runtime::init_on_ui_thread: cargo test workers are multi-threaded
    // and the UI thread id is process-global. Unsync reactive APIs work without
    // enforcement until init_on_ui_thread is called.

    let arena = Rc::new(RefCell::new(ElementArena::new()));
    let dirty = Rc::new(RefCell::new(DirtySet::new()));
    let text_el = arena.borrow_mut().insert_primitive();
    let sibling = arena.borrow_mut().insert_primitive();

    let text = RwSignal::new("hello".to_string());
    let mut registry = BindingRegistry::new();
    let binding = registry.bind_signal(
        arena.clone(),
        dirty.clone(),
        ReactiveBinding {
            element: text_el,
            flags: DirtyFlags::PAINT,
        },
        text,
    );

    // Initial effect run marks the bound element once.
    assert_eq!(registry.run_count(binding), Some(1));
    assert!(dirty.borrow().get(text_el).contains(DirtyFlags::PAINT));
    assert_eq!(dirty.borrow().get(sibling), DirtyFlags::empty());

    dirty.borrow_mut().clear(text_el);
    text.set("world".to_string());

    assert_eq!(registry.run_count(binding), Some(2));
    assert!(dirty.borrow().get(text_el).contains(DirtyFlags::PAINT));
    assert_eq!(
        dirty.borrow().get(sibling),
        DirtyFlags::empty(),
        "sibling must not rebuild/invalidate on unrelated signal"
    );
}

#[test]
fn unbind_stops_updates() {
    let arena = Rc::new(RefCell::new(ElementArena::new()));
    let dirty = Rc::new(RefCell::new(DirtySet::new()));
    let el = arena.borrow_mut().insert_primitive();
    let signal = RwSignal::new(0i32);

    let mut registry = BindingRegistry::new();
    let id = registry.bind_signal(
        arena.clone(),
        dirty.clone(),
        ReactiveBinding {
            element: el,
            flags: DirtyFlags::STATE,
        },
        signal,
    );
    assert_eq!(registry.run_count(id), Some(1));
    assert!(registry.unbind(id));

    dirty.borrow_mut().clear(el);
    signal.set(1);
    assert_eq!(dirty.borrow().get(el), DirtyFlags::empty());
    assert!(registry.run_count(id).is_none());
}
