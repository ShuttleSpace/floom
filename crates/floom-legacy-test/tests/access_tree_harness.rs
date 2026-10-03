use floom::accessibility::AccessRole;
use floom::headless::{HeadlessHarness, TestRoot};
use floom::views::{Button, Decorators};
use floom::IntoView;

#[test]
fn harness_exposes_access_tree_with_button_label() {
    let root = TestRoot::new();
    let view = Button::new("Save")
        .style(|s| s.size(80.0, 32.0))
        .into_view();
    let mut harness = HeadlessHarness::new_with_size(root, view, 200.0, 100.0);
    harness.rebuild();
    let tree = harness.access_tree();
    assert!(
        tree.iter()
            .any(|n| n.label.as_deref() == Some("Save") || n.role == AccessRole::Button),
        "expected a button node in access tree, got len={}",
        tree.len()
    );
}
