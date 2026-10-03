use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_cupertino_icon::CupertinoIcon;

#[test]
fn icon_exposes_accessible_builder_api_and_theme_style() {
    let theme = MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino);
    let icon = CupertinoIcon::new("home").label("Home");
    let style = icon.with_theme(theme.clone()).resolved_style();

    assert_eq!(style.design_system, "cupertino");
    assert_eq!(style.color, theme.colors().on_surface);
    assert_eq!(style.size, 22.0);
}
