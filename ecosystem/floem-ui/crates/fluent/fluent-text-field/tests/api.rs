use floem_core_theme::theme::{MultiTheme, Theme};
use floem_widget_fluent_text_field::FluentTextField;

#[test]
fn text_field_exposes_neutral_contract_api() {
    let field = FluentTextField::new()
        .placeholder("Name")
        .initial_value("Ada")
        .password(true)
        .max_length(8)
        .disabled(true);

    assert_eq!(field.props().placeholder, "Name");
    assert_eq!(field.props().value, "Ada");
    assert!(field.props().is_password);
    assert_eq!(field.props().max_length, Some(8));
    assert!(field.props().disabled);
    assert_eq!(field.resolved_style().design_system, "fluent");
}

#[test]
fn text_field_style_resolves_from_fluent_theme() {
    let theme = MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent);
    let style = FluentTextField::new()
        .with_theme(theme.clone())
        .resolved_style();

    assert_eq!(style.border_color, theme.colors().outline);
    assert_eq!(style.text_color, theme.colors().on_surface);
    assert_eq!(style.height, 32.0);
    assert_eq!(style.border_radius, 4.0);
}
