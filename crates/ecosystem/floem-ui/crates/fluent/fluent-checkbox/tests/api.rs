use floem_core_theme::theme::{MultiTheme, Theme};
use floem_widget_fluent_checkbox::FluentCheckbox;

#[test]
fn checkbox_exposes_neutral_contract_api() {
    let checkbox = FluentCheckbox::new("Accept").checked(true).disabled(true);

    assert_eq!(checkbox.props().label, "Accept");
    assert!(checkbox.props().checked);
    assert!(checkbox.props().disabled);
    assert_eq!(checkbox.resolved_style().design_system, "fluent");
}

#[test]
fn checkbox_style_resolves_from_fluent_theme() {
    let theme = MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent);
    let style = FluentCheckbox::new("Accept")
        .checked(true)
        .with_theme(theme.clone())
        .resolved_style();

    assert_eq!(style.container_color, theme.colors().primary);
    assert_eq!(style.mark_color, theme.colors().on_primary);
    assert_eq!(style.size, 20.0);
    assert_eq!(style.border_radius, 4.0);
}
