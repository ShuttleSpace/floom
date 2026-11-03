
use std::collections::HashMap;
use std::sync::Arc;

use floom_core::Widget;

use floom_widget_md_defaults_provider::MaterialDefaultsProvider;

/// The props report what the caller set.
#[test]
fn the_props_report_the_map() {
    let mut map = HashMap::new();
    map.insert("density".to_string(), "compact".to_string());
    let provider = MaterialDefaultsProvider::new().defaults(map.clone());
    assert_eq!(provider.props().defaults, map);
}

/// The handle the descendants read holds the map.
#[test]
fn the_shared_handle_carries_the_map() {
    let mut map = HashMap::new();
    map.insert("density".to_string(), "compact".to_string());
    let provider = MaterialDefaultsProvider::new().defaults(map.clone());
    let handle = provider.defaults_signal();
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
    let provider = MaterialDefaultsProvider::new();
    let first = provider.defaults_signal();
    let second = provider.defaults_signal();
    assert!(
        Arc::ptr_eq(&first, &second),
        "the handle is shared, not copied"
    );
}

/// The subtree is mounted inside the provider.
#[test]
fn the_subtree_is_mounted() {
    let stack = MaterialDefaultsProvider::new()
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
