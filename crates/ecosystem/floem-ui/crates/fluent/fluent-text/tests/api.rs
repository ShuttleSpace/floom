use floem_core_theme::theme::{MultiTheme, Theme};
use floem_widget_fluent_text::FluentText;

#[test]
fn text_exposes_neutral_contract_api() {
    let text = FluentText::new("Body").max_lines(2).selectable(true);

    assert_eq!(text.props().content, "Body");
    assert_eq!(text.props().max_lines, Some(2));
    assert!(text.props().is_selectable);
    assert_eq!(text.resolved_style().design_system, "fluent");
}

#[test]
fn text_style_resolves_from_fluent_theme() {
    let theme = MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent);
    let style = FluentText::new("Body")
        .with_theme(theme.clone())
        .resolved_style();

    assert_eq!(style.color, theme.colors().on_surface);
    assert_eq!(style.font_size, theme.typography().body.medium.font_size);
    assert_eq!(
        style.line_height,
        theme.typography().body.medium.line_height
    );
}
