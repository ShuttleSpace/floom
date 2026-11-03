//! The Cupertino button's *contract* API: the props a caller sets and reads
//! back.

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
