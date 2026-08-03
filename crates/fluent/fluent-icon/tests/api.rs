use floem_core_theme::theme::{FluentTheme, Theme};
use floem_widget_fluent_icon::FluentIcon;

#[test]
fn icon_exposes_accessible_builder_api_and_theme_style() {
    let theme = FluentTheme::new();
    let icon = FluentIcon::new("home").label("Home");
    let style = icon.with_theme(theme.clone()).resolved_style();

    assert_eq!(style.design_system, "fluent");
    assert_eq!(style.color, theme.colors().on_surface);
    assert_eq!(style.size, 20.0);
}
