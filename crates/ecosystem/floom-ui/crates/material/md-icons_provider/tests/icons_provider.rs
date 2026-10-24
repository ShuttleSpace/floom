//! Material icons_provider: the map it shares, and the subtree it wraps.
//!
//! ## The claim
//!
//! A provider owns a map of overrides and hands its descendants a handle to
//! it. v1 held it in an `RwSignal<HashMap<String, String>>`; 2.0 holds it in
//! a `SharedMap`, because an `RwSignal` is `Rc`-based and cannot cross a
//! `Send + Sync` slot boundary. The handle is the *same* one a descendant
//! reads -- not a copy.

use std::collections::HashMap;
use std::sync::Arc;

use floom_core::Widget;

use floom_widget_md_icons_provider::MaterialIconsProvider;

/// The props report what the caller set.
#[test]
fn the_props_report_the_map() {
    let mut map = HashMap::new();
    map.insert("density".to_string(), "compact".to_string());
    let provider = MaterialIconsProvider::new().icons(map.clone());
    assert_eq!(provider.props().icons, map);
}

/// The handle the descendants read holds the map.
#[test]
fn the_shared_handle_carries_the_map() {
    let mut map = HashMap::new();
    map.insert("density".to_string(), "compact".to_string());
    let provider = MaterialIconsProvider::new().icons(map.clone());
    let handle = provider.icons_signal();
    assert_eq!(
        *handle.lock().unwrap(),
        map,
        "the handle the descendants read holds the map"
    );
}

/// Two calls hand over the *same* handle, so a descendant that wrote it would
/// be seen by the next one that reads it.
#[test]
fn the_handle_is_shared() {
    let provider = MaterialIconsProvider::new();
    let first = provider.icons_signal();
    let second = provider.icons_signal();
    assert!(
        Arc::ptr_eq(&first, &second),
        "the handle is shared, not copied"
    );
}

/// The subtree is mounted inside the provider.
#[test]
fn the_subtree_is_mounted() {
    let stack = MaterialIconsProvider::new()
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
