use floem_focus::{FocusManager, FocusScopeId};
use floem_types::ElementId;

fn eid(i: u32) -> ElementId {
    ElementId::new(i, 1)
}

#[test]
fn tab_and_shift_tab_cycle() {
    let mut fm = FocusManager::new();
    let scope = fm.create_scope(None);
    fm.register(scope, eid(1));
    fm.register(scope, eid(2));
    fm.register(scope, eid(3));

    assert_eq!(fm.tab(), Some(eid(1)));
    assert_eq!(fm.tab(), Some(eid(2)));
    assert_eq!(fm.tab(), Some(eid(3)));
    assert_eq!(fm.tab(), Some(eid(1)));
    assert_eq!(fm.shift_tab(), Some(eid(3)));
    assert_eq!(fm.shift_tab(), Some(eid(2)));
}

#[test]
fn nested_scope_activation() {
    let mut fm = FocusManager::new();
    let root = fm.create_scope(None);
    let nested = fm.create_scope(Some(root));
    fm.register(root, eid(1));
    fm.register(nested, eid(10));
    fm.register(nested, eid(11));

    fm.set_active_scope(nested);
    assert_eq!(fm.tab(), Some(eid(10)));
    assert_eq!(fm.tab(), Some(eid(11)));
}

#[test]
fn platform_focus_sync() {
    let mut fm = FocusManager::new();
    let scope = fm.create_scope(None);
    fm.register(scope, eid(5));
    fm.focus(eid(5));
    assert_eq!(fm.platform_focused, Some(eid(5)));
    fm.on_platform_focus(false);
    assert!(fm.platform_focused.is_none());
    fm.on_platform_focus(true);
    assert_eq!(fm.platform_focused, Some(eid(5)));
}

#[test]
fn unused_scope_id_type() {
    let _ = FocusScopeId(0);
}

#[test]
fn unregistered_nodes_leave_the_traversal_order() {
    let mut fm = FocusManager::new();
    let scope = fm.create_scope(None);
    fm.register(scope, eid(1));
    fm.register(scope, eid(2));
    fm.register(scope, eid(3));

    fm.unregister(eid(2));

    assert!(!fm.is_registered(eid(2)));
    assert_eq!(fm.tab(), Some(eid(1)));
    // Skips the removed node instead of stalling on an empty slot.
    assert_eq!(fm.tab(), Some(eid(3)));
    assert_eq!(fm.tab(), Some(eid(1)));
}

#[test]
fn unregistering_the_focused_node_clears_focus() {
    let mut fm = FocusManager::new();
    let scope = fm.create_scope(None);
    fm.register(scope, eid(1));
    fm.register(scope, eid(2));
    fm.focus(eid(2));
    assert_eq!(fm.focused(), Some(eid(2)));

    fm.unregister(eid(2));

    assert_eq!(fm.focused(), None, "focus must not outlive its node");
    assert_eq!(fm.platform_focused, None);
}

#[test]
fn registering_twice_does_not_duplicate() {
    let mut fm = FocusManager::new();
    let scope = fm.create_scope(None);
    fm.register(scope, eid(1));
    fm.register(scope, eid(1));
    assert_eq!(fm.tab(), Some(eid(1)));
    assert_eq!(fm.tab(), Some(eid(1)), "one visit per node per cycle");
}
