use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_fluent_switch::FluentSwitch;

#[test]
fn switch_exposes_neutral_contract_api() {
    let switch = FluentSwitch::new().checked(true).disabled(true);

    assert!(switch.props().checked);
    assert!(switch.props().disabled);
    assert_eq!(switch.resolved_style().design_system, "fluent");
}

#[test]
fn switch_style_resolves_from_fluent_theme() {
    let theme = MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent);
    let style = FluentSwitch::new()
        .checked(true)
        .with_theme(theme.clone())
        .resolved_style();

    assert_eq!(style.track_color, theme.colors().primary);
    assert_eq!(style.thumb_color, theme.colors().on_primary);
    assert_eq!(style.width, 40.0);
    assert_eq!(style.height, 20.0);
    assert_eq!(style.thumb_offset, 20.0);
}
