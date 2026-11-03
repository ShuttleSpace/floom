//! Cupertino locale_provider: the value it shares, and the subtree it wraps.
//!

use std::sync::Arc;

use floom_core::Widget;

use floom_core_theme::props::locale_provider::LocaleProviderProps;

use floom_widget_cupertino_locale_provider::CupertinoLocaleProvider;

/// The props report what the caller set.
#[test]
fn the_props_report_the_value() {
    let provider = CupertinoLocaleProvider::new().locale("en-GB");
    assert_eq!(provider.props().locale, "en-GB");
}

/// The handle the descendants read holds the value.
#[test]
fn the_shared_handle_carries_the_value() {
    let provider = CupertinoLocaleProvider::new().locale("en-GB");
    let handle = provider.locale_signal();
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
    let provider = CupertinoLocaleProvider::new();
    let first = provider.locale_signal();
    let second = provider.locale_signal();
    assert!(
        Arc::ptr_eq(&first, &second),
        "the handle is shared, not copied"
    );
}

/// The subtree is mounted inside the provider.
#[test]
fn the_subtree_is_mounted() {
    let stack = CupertinoLocaleProvider::new()
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
