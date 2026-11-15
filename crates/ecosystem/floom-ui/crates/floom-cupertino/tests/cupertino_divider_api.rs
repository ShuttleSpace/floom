#![cfg(feature = "divider")]
use floom_core_theme::props::divider::DividerOrientation;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_cupertino::widgets::divider::CupertinoDivider;

#[test]
fn divider_exposes_orientation_inset_and_theme_style() {
    let theme = MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino);
    let divider = CupertinoDivider::new()
        .orientation(DividerOrientation::Vertical)
        .inset(16.0);
    let style = divider.with_theme(theme.clone()).resolved_style();

    assert_eq!(style.design_system, "cupertino");
    assert_eq!(style.color, theme.colors().outline_variant);
    assert_eq!(style.thickness, 1.0);
    assert_eq!(style.inset, 16.0);
    assert_eq!(style.orientation, DividerOrientation::Vertical);
}
