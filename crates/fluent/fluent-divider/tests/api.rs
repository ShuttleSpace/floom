use floem_core_theme::props::divider::DividerOrientation;
use floem_core_theme::theme::{FluentTheme, Theme};
use floem_widget_fluent_divider::FluentDivider;

#[test]
fn divider_exposes_orientation_inset_and_theme_style() {
    let theme = FluentTheme::new();
    let divider = FluentDivider::new()
        .orientation(DividerOrientation::Vertical)
        .inset(16.0);
    let style = divider.with_theme(theme.clone()).resolved_style();

    assert_eq!(style.design_system, "fluent");
    assert_eq!(style.color, theme.colors().outline_variant);
    assert_eq!(style.thickness, 1.0);
    assert_eq!(style.inset, 16.0);
    assert_eq!(style.orientation, DividerOrientation::Vertical);
}
