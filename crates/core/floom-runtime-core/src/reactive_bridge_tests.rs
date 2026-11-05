// SPDX-License-Identifier: MIT
use super::{BindingRegistry, ReactiveBinding, SignalSource};
use crate::arena::ElementArena;
use crate::dirty::{DirtyFlags, DirtySet};
use floom_signal::{RwSignal, SignalUpdate};
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

#[test]
fn the_update_path_names_the_signal_that_fired() {
    // "Why did this update?" needs the *source*, not just the fact that
    // something went dirty: two signals can both target one element, and the
    // diagnostic that cannot tell them apart is noise.
    let arena = Rc::new(RefCell::new(ElementArena::new()));
    let dirty = Rc::new(RefCell::new(DirtySet::new()));
    let el = arena.borrow_mut().insert_primitive();

    let label = RwSignal::new("hello".to_string());
    let count = RwSignal::new(0u32);
    let mut registry = BindingRegistry::new();

    let label_binding = registry.bind_signal_with_source(
        arena.clone(),
        dirty.clone(),
        ReactiveBinding {
            element: el,
            flags: DirtyFlags::PAINT,
        },
        label,
        SignalSource {
            signal: label.id(),
            label: "heading_text".into(),
            property: "label".into(),
        },
    );
    let _count_binding = registry.bind_signal_with_source(
        arena.clone(),
        dirty.clone(),
        ReactiveBinding {
            element: el,
            flags: DirtyFlags::STYLE,
        },
        count,
        SignalSource {
            signal: count.id(),
            label: "progress_value".into(),
            property: "background".into(),
        },
    );

    dirty.borrow_mut().clear(el);
    count.set(3);

    let reports = registry.reports_for_element(el);
    assert_eq!(reports.len(), 2, "two bindings target the element");
    let fired = reports
        .iter()
        .find(|r| r.runs >= 2)
        .expect("exactly one binding ran twice");
    assert_eq!(
        fired.source.as_ref().map(|s| s.label.as_str()),
        Some("progress_value"),
        "the update path names the signal that actually fired"
    );
    assert_eq!(
        fired.source.as_ref().map(|s| s.property.as_str()),
        Some("background")
    );
    let _ = label_binding;
}

#[test]
fn a_bound_element_without_source_metadata_still_reports() {
    // Bindings made before the diagnostic existed carry no source; the report
    // must not crash or drop them — it just says "unknown source".
    let arena = Rc::new(RefCell::new(ElementArena::new()));
    let dirty = Rc::new(RefCell::new(DirtySet::new()));
    let el = arena.borrow_mut().insert_primitive();
    let text = RwSignal::new("x".to_string());
    let mut registry = BindingRegistry::new();
    let binding = registry.bind_signal(
        arena.clone(),
        dirty.clone(),
        ReactiveBinding {
            element: el,
            flags: DirtyFlags::PAINT,
        },
        text,
    );

    let reports = registry.reports_for_element(el);
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].id, binding);
    assert!(reports[0].source.is_none());
}

#[test]
fn an_unbound_element_reports_no_bindings() {
    let arena = Rc::new(RefCell::new(ElementArena::new()));
    let el = arena.borrow_mut().insert_primitive();
    let registry = BindingRegistry::new();
    assert!(registry.reports_for_element(el).is_empty());
}
