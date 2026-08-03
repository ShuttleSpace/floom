use floem_core_theme::theme::{CupertinoTheme, Theme};
use floem_widget_cupertino_checkbox::CupertinoCheckbox;

#[test]
fn checkbox_exposes_neutral_contract_api() {
    let checkbox = CupertinoCheckbox::new("Accept")
        .checked(true)
        .disabled(true);

    assert_eq!(checkbox.props().label, "Accept");
    assert!(checkbox.props().checked);
    assert!(checkbox.props().disabled);
    assert_eq!(checkbox.resolved_style().design_system, "cupertino");
}

#[test]
fn checkbox_style_resolves_from_cupertino_theme() {
    let theme = CupertinoTheme::new();
    let style = CupertinoCheckbox::new("Accept")
        .checked(true)
        .with_theme(theme.clone())
        .resolved_style();

    assert_eq!(style.container_color, theme.colors().primary);
    assert_eq!(style.mark_color, theme.colors().on_primary);
    assert_eq!(style.size, 20.0);
    assert_eq!(style.border_radius, 6.0);
}
