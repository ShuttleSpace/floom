use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_fluent_menu::FluentMenu;

///
/// The design system has to be *selected*, not merely built: `MultiTheme`
/// holds all three palettes and reads the active one, and `activate` is a
/// documented no-op. Without this the test would read Material's tokens and
/// assert this kit's numbers.
fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

#[test]
fn menu_exposes_neutral_contract_api() {
    let menu = FluentMenu::new().item("Open", || {}).open(true);

    assert_eq!(menu.props().items.len(), 1);
    assert_eq!(menu.props().items[0].label, "Open");
    assert!(menu.props().open);
    assert_eq!(menu.resolved_style().design_system, "fluent");
}

#[test]
fn menu_style_resolves_from_fluent_theme() {
    let theme = theme();
    let style = FluentMenu::new().with_theme(theme.clone()).resolved_style();

    assert_eq!(style.container_color, theme.colors().surface);
    assert_eq!(style.text_color, theme.colors().on_surface);
    assert_eq!(style.min_width, 120.0);
    assert_eq!(style.item_height, 32.0);
}
