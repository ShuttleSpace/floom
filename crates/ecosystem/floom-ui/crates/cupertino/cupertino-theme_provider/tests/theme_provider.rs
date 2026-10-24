//! Cupertino theme_provider: the value it shares, and the subtree it wraps.
//!
//! ## The claim
//!
//! A provider owns a value and hands its descendants a handle to it. v1 held it
//! in an `RwSignal`; 2.0 holds it in a shared flag, because an `RwSignal` is
//! `Rc`-based and cannot cross a `Send + Sync` slot boundary. The handle is
//! therefore the *same* one a descendant reads -- not a copy -- and the
//! assertions are about that.

use std::sync::Arc;

use floom_core::Widget;

use floom_core_theme::props::theme_provider::ThemeProviderProps;

use floom_widget_cupertino_theme_provider::CupertinoThemeProvider;

/// The props report what the caller set.
#[test]
fn the_props_report_the_value() {
    let provider = CupertinoThemeProvider::new().dark(true);
    assert_eq!(provider.props().dark, true);
}

/// The handle the descendants read holds the value.
#[test]
fn the_shared_handle_carries_the_value() {
    let provider = CupertinoThemeProvider::new().dark(true);
    let handle = provider.dark_signal();
    assert!(
        floom_widgets::text_input::read_flag(&handle),
        "the handle the descendants read holds the value"
    );
}

/// Two calls hand over the *same* handle, so a descendant that wrote it would
/// be seen by the next one that reads it.
#[test]
fn the_handle_is_shared() {
    let provider = CupertinoThemeProvider::new();
    let first = provider.dark_signal();
    let second = provider.dark_signal();
    assert!(
        Arc::ptr_eq(&first, &second),
        "the handle is shared, not copied"
    );
}

/// The subtree is mounted inside the provider.
#[test]
fn the_subtree_is_mounted() {
    let stack = CupertinoThemeProvider::new()
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
