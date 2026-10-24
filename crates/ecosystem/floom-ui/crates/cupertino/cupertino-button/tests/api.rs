//! The Cupertino button's *contract* API: the props a caller sets and reads
//! back.
//!
//! This file is the v1 `api.rs`, carried forward. v1's version asserted two
//! things this port keeps:
//!
//! * the neutral contract -- label, variant, size, disabled and loading all
//!   round-trip through `props()`;
//! * the style resolving from *this* design system's theme, with the
//!   Cupertino-specific numbers (`44.0` for the button height,
//!   `radius().lg` for the corner radius) rather than another system's.
//!
//! The one thing that changed is the constructor: v1 took the label in
//! `new("Save")`. The ported kits use `new()` plus a builder, and so does this
//! one.
//!
//! ## Why the design system is selected in the test
//!
//! `MultiTheme::new_for(DesignSystem::Cupertino)` does *not* select a design
//! system -- it builds all three and `activate` is a documented no-op. The
//! active one is read at read time from a thread-local. So a test that wants
//! Cupertino's own numbers has to select Cupertino, and
//! `set_design_system` is the way to do it. It writes a thread-local, so the
//! selection is this test's alone.

use floom_core_theme::global_theme::{set_design_system, DesignSystem};
use floom_core_theme::props::button::{ButtonSize, ButtonVariant};
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_cupertino_button::CupertinoButton;

/// Selects Cupertino and hands back its theme.
fn theme() -> MultiTheme {
    set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

#[test]
fn button_exposes_neutral_contract_api() {
    let button = CupertinoButton::new()
        .label("Save")
        .variant(ButtonVariant::Secondary)
        .size(ButtonSize::Large)
        .disabled(true)
        .loading(true);

    assert_eq!(button.props().label, "Save");
    assert_eq!(button.props().variant, ButtonVariant::Secondary);
    assert_eq!(button.props().size, ButtonSize::Large);
    assert!(button.props().disabled);
    assert!(button.props().loading);
    assert_eq!(button.resolved_style().design_system, "cupertino");
}

#[test]
fn button_style_resolves_from_cupertino_theme() {
    let theme = theme();
    let style = CupertinoButton::new()
        .label("Save")
        .with_theme(theme.clone())
        .resolved_style();

    assert_eq!(style.background, theme.colors().primary);
    assert_eq!(style.foreground, theme.colors().on_primary);
    assert_eq!(
        style.height, 44.0,
        "Cupertino's button token height, not another system's"
    );
    assert_eq!(style.border_radius, theme.radius().lg);
}
