//! Cupertino date_adapter_provider: the value it shares, and the subtree it wraps.
//!

use std::sync::Arc;

use floom_core::Widget;

use floom_core_theme::props::date_adapter_provider::DateAdapterProviderProps;

use floom_widget_cupertino_date_adapter_provider::CupertinoDateAdapterProvider;

/// The props report what the caller set.
#[test]
fn the_props_report_the_value() {
    let provider = CupertinoDateAdapterProvider::new().adapter("en-GB");
    assert_eq!(provider.props().adapter, "en-GB");
}

/// The handle the descendants read holds the value.
#[test]
fn the_shared_handle_carries_the_value() {
    let provider = CupertinoDateAdapterProvider::new().adapter("en-GB");
    let handle = provider.adapter_signal();
    assert_eq!(
        handle.lock().unwrap().clone(),
        "en-GB",
        "the handle the descendants read holds the value"
    );
}

/// Two calls hand over the *same* handle, so a descendant that wrote it would
/// be seen by the next one that reads it.
#[test]
fn the_handle_is_shared() {
    let provider = CupertinoDateAdapterProvider::new();
    let first = provider.adapter_signal();
    let second = provider.adapter_signal();
    assert!(
        Arc::ptr_eq(&first, &second),
        "the handle is shared, not copied"
    );
}

/// The subtree is mounted inside the provider.
#[test]
fn the_subtree_is_mounted() {
    let stack = CupertinoDateAdapterProvider::new()
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
