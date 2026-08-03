use floem_core_theme::theme::{MaterialTheme, Theme};
use floem_widget_md_icon::MdIcon;

#[test]
fn icon_exposes_accessible_builder_api_and_theme_style() {
    let theme = MaterialTheme::new();
    let icon = MdIcon::new("home").size(20.0).label("Home");
    let style = icon.with_theme(theme.clone()).resolved_style();

    assert_eq!(style.color, theme.colors().on_surface_variant);
    assert_eq!(style.size, 20.0);
}
