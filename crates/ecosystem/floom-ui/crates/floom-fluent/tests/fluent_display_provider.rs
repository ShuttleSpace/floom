#![cfg(feature = "display_provider")]
//! Fluent display_provider: the value it shares, and the subtree it wraps.

use std::sync::Arc;

use floom_core::Widget;

use floom_core_theme::props::display_provider::DisplayProviderProps;

use floom_fluent::widgets::display_provider::FluentDisplayProvider;

#[test]
fn the_props_report_the_value() {
    let provider = FluentDisplayProvider::new().dense(true);
    assert_eq!(provider.props().dense, true);
}

#[test]
fn the_shared_handle_carries_the_value() {
    let provider = FluentDisplayProvider::new().dense(true);
    let handle = provider.density_signal();
    assert!(
        floom_widgets::text_input::read_flag(&handle),
        "the handle the descendants read holds the value"
    );
}

#[test]
fn the_handle_is_shared() {
    let provider = FluentDisplayProvider::new();
    let first = provider.density_signal();
    let second = provider.density_signal();
    assert!(
        Arc::ptr_eq(&first, &second),
        "the handle is shared, not copied"
    );
}

#[test]
fn the_subtree_is_mounted() {
    let stack = FluentDisplayProvider::new()
        .child(|| Box::new(floom_widgets::Text::new("body")))
        .view();
    assert_eq!(stack.child_count(), 1, "the caller's subtree is inside");
    assert!(
        stack.children()[0]
            .build()
            .as_any()
            .is::<floom_widgets::Text>(),
        "and it is the caller's own content"
    );
}
