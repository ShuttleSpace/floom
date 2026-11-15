#![cfg(feature = "locale_provider")]
//! Fluent locale_provider: the value it shares, and the subtree it wraps.

use std::sync::Arc;

use floom_core::Widget;

use floom_core_theme::props::locale_provider::LocaleProviderProps;

use floom_fluent::widgets::locale_provider::FluentLocaleProvider;

#[test]
fn the_props_report_the_value() {
    let provider = FluentLocaleProvider::new().locale("en-GB");
    assert_eq!(provider.props().locale, "en-GB");
}

#[test]
fn the_shared_handle_carries_the_value() {
    let provider = FluentLocaleProvider::new().locale("en-GB");
    let handle = provider.locale_signal();
    assert_eq!(
        handle.lock().unwrap().clone(),
        "en-GB",
        "the handle the descendants read holds the value"
    );
}

#[test]
fn the_handle_is_shared() {
    let provider = FluentLocaleProvider::new();
    let first = provider.locale_signal();
    let second = provider.locale_signal();
    assert!(
        Arc::ptr_eq(&first, &second),
        "the handle is shared, not copied"
    );
}

#[test]
fn the_subtree_is_mounted() {
    let stack = FluentLocaleProvider::new()
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
