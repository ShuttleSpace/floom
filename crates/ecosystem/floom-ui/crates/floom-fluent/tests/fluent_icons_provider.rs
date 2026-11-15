#![cfg(feature = "icons_provider")]
//! Fluent icons_provider: the map it shares, and the subtree it wraps.

use std::collections::HashMap;
use std::sync::Arc;

use floom_core::Widget;

use floom_fluent::widgets::icons_provider::FluentIconsProvider;

#[test]
fn the_props_report_the_map() {
    let mut map = HashMap::new();
    map.insert("density".to_string(), "compact".to_string());
    let provider = FluentIconsProvider::new().icons(map.clone());
    assert_eq!(provider.props().icons, map);
}

#[test]
fn the_shared_handle_carries_the_map() {
    let mut map = HashMap::new();
    map.insert("density".to_string(), "compact".to_string());
    let provider = FluentIconsProvider::new().icons(map.clone());
    let handle = provider.icons_signal();
    assert_eq!(
        *handle.lock().unwrap(),
        map,
        "the handle the descendants read holds the map"
    );
}

#[test]
fn the_handle_is_shared() {
    let provider = FluentIconsProvider::new();
    let first = provider.icons_signal();
    let second = provider.icons_signal();
    assert!(
        Arc::ptr_eq(&first, &second),
        "the handle is shared, not copied"
    );
}

#[test]
fn the_subtree_is_mounted() {
    let stack = FluentIconsProvider::new()
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
