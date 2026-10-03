use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_cupertino_text::CupertinoText;

#[test]
fn text_exposes_neutral_contract_api() {
    let text = CupertinoText::new("Body").max_lines(2).selectable(true);

    assert_eq!(text.props().content, "Body");
    assert_eq!(text.props().max_lines, Some(2));
    assert!(text.props().is_selectable);
    assert_eq!(text.resolved_style().design_system, "cupertino");
}

#[test]
fn text_style_resolves_from_cupertino_theme() {
    let theme = MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino);
    let style = CupertinoText::new("Body")
        .with_theme(theme.clone())
        .resolved_style();

    assert_eq!(style.color, theme.colors().on_surface);
    assert_eq!(style.font_size, theme.typography().body.medium.font_size);
    assert_eq!(
        style.line_height,
        theme.typography().body.medium.line_height
    );
}
