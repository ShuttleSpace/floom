#![cfg(feature = "defaults_provider")]
//! Fluent defaults_provider: the map it shares, and the subtree it wraps.

use std::collections::HashMap;
use std::sync::Arc;

use floom_core::Widget;

use floom_fluent::widgets::defaults_provider::FluentDefaultsProvider;

#[test]
fn the_props_report_the_map() {
    let mut map = HashMap::new();
    map.insert("density".to_string(), "compact".to_string());
    let provider = FluentDefaultsProvider::new().defaults(map.clone());
    assert_eq!(provider.props().defaults, map);
}

#[test]
fn the_shared_handle_carries_the_map() {
    let mut map = HashMap::new();
    map.insert("density".to_string(), "compact".to_string());
    let provider = FluentDefaultsProvider::new().defaults(map.clone());
    let handle = provider.defaults_signal();
    assert_eq!(
        *handle.lock().unwrap(),
        map,
        "the handle the descendants read holds the map"
    );
}

#[test]
fn the_handle_is_shared() {
    let provider = FluentDefaultsProvider::new();
    let first = provider.defaults_signal();
    let second = provider.defaults_signal();
    assert!(
        Arc::ptr_eq(&first, &second),
        "the handle is shared, not copied"
    );
}

#[test]
fn the_subtree_is_mounted() {
    let stack = FluentDefaultsProvider::new()
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
