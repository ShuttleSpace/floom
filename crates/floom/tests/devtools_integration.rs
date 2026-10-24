//! The one-line devtools integration, end to end.
//!
//! The promise is one line in and a keystroke away. That promise has two halves
//! that can each fail on their own: `attach` mounting a panel the host can
//! actually see, and the shortcut reaching that panel before the tree does.
//! Both are checkable without a window — the host mounts overlays headlessly,
//! and a key event is a value.
//!
//! Gated on the feature because `floom::devtools` is: without it this file
//! names a module that does not exist, so a plain `cargo test --workspace`
//! fails to compile instead of skipping. The feature is off by default, so
//! `cargo test -p floom --features devtools` is what runs these.

#![cfg(feature = "devtools")]

use floom_app::WidgetHost;
use floom_core::Widget;
use floom_layout::LayoutStyle;
use floom_types::{Key, KeyEvent, KeyPhase, Modifiers, NamedKey};

/// A host with one widget mounted, the way an app's `build` closure leaves it.
fn host() -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(floom_types::Size::new(800.0, 600.0));
    host.mount_root(LayoutStyle::default(), floom_widgets::Text::new("app"));
    host
}

#[test]
fn one_line_mounts_the_panel_as_an_overlay() {
    let mut host = host();
    assert_eq!(host.overlay_count(), 0);

    // The one line.
    let devtools = floom::devtools::attach(&mut host, |state| {
        vec![floom::devtools::ReadoutRow::new(
            "visible",
            state.visible().to_string(),
        )]
    });

    // The panel is above the app's tree, not part of it.
    assert_eq!(host.overlay_count(), 1);
    // And it starts hidden, so an app that never opens it pays nothing.
    assert!(!devtools.visible());
}

#[test]
fn the_panel_docks_inside_the_window() {
    let mut host = host();
    let _ = floom::devtools::attach(&mut host, |_| Vec::new());
    // One overlay, sized to the window the host was given.
    assert_eq!(host.overlay_count(), 1);
    let bounds = host.overlay_bounds(floom_types::OverlayHandle::new(0));
    assert!(bounds.width() > 0.0 && bounds.width() <= 800.0);
    assert_eq!(bounds.height(), 600.0);
}

#[test]
fn show_false_is_the_default_and_show_true_opens_it() {
    let mut host = host();
    let devtools = floom::devtools::attach(&mut host, |_| Vec::new());
    assert!(!devtools.visible(), "hidden until asked");
    devtools.show();
    assert!(devtools.visible());
    devtools.hide();
    assert!(!devtools.visible());
    devtools.toggle();
    assert!(devtools.visible());
}

#[test]
fn the_shortcut_reaches_the_panel() {
    let mut host = host();
    let mut devtools = floom::devtools::attach(&mut host, |_| Vec::new());
    let f12 = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Named(NamedKey::Function(12)),
        modifiers: Modifiers::NONE,
    };
    // Consumed, and the panel opened.
    assert!(devtools.handle_key(&f12));
    assert!(devtools.visible());
    // Consumed again, and the panel closed.
    assert!(devtools.handle_key(&f12));
    assert!(!devtools.visible());
}

#[test]
fn an_unrelated_key_falls_through_to_the_app() {
    let mut host = host();
    let mut devtools = floom::devtools::attach(&mut host, |_| Vec::new());
    let tab = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Named(NamedKey::Tab),
        modifiers: Modifiers::NONE,
    };
    // Not consumed, so a text field keeps typing.
    assert!(!devtools.handle_key(&tab));
    assert!(!devtools.visible());
}

#[test]
fn a_shown_panel_actually_mounts_its_rows() {
    let mut host = host();
    let mut devtools = floom::devtools::attach(&mut host, |state| {
        vec![floom::devtools::ReadoutRow::new(
            "tab",
            format!("{:?}", state.tab()),
        )]
    });
    devtools.show();
    // The panel is a widget: shown, it has children; hidden, it has none.
    let overlay = host
        .overlay_widget(floom_types::OverlayHandle::new(0))
        .expect("the overlay the one line mounted");
    assert!(!overlay.children().is_empty());
}
