use floem::accessibility::{AccessAction, AccessRole};
use floem::headless::{HeadlessHarness, TestRoot};
use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
use floem::views::{button, Decorators};

#[test]
fn find_by_accessible_label_clicks_button() {
    let root = TestRoot::new();
    let clicked = RwSignal::new(false);
    let view = button("Go").action(move || clicked.set(true));
    let mut harness = HeadlessHarness::new_with_size(root, view, 200.0, 80.0);
    harness.rebuild();
    let id = harness
        .find_by_accessible_label("Go")
        .expect("label Go");
    assert!(harness.perform_access_action(id, AccessAction::Default));
    harness.process_update_messages();
    assert!(clicked.get());
}

#[test]
fn find_by_role_returns_button() {
    let root = TestRoot::new();
    let view = button("X");
    let mut harness = HeadlessHarness::new_with_size(root, view, 200.0, 80.0);
    harness.rebuild();
    let ids = harness.find_by_accessible_role(AccessRole::Button);
    assert!(!ids.is_empty());
}
