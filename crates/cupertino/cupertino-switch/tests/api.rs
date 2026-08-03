use floem_core_theme::theme::{CupertinoTheme, Theme};
use floem_widget_cupertino_switch::CupertinoSwitch;

#[test]
fn switch_exposes_neutral_contract_api() {
    let switch = CupertinoSwitch::new().checked(true).disabled(true);

    assert!(switch.props().checked);
    assert!(switch.props().disabled);
    assert_eq!(switch.resolved_style().design_system, "cupertino");
}

#[test]
fn switch_style_resolves_from_cupertino_theme() {
    let theme = CupertinoTheme::new();
    let style = CupertinoSwitch::new()
        .checked(true)
        .with_theme(theme.clone())
        .resolved_style();

    assert_eq!(style.track_color, theme.colors().primary);
    assert_eq!(style.thumb_color, theme.colors().on_primary);
    assert_eq!(style.width, 51.0);
    assert_eq!(style.height, 31.0);
    assert_eq!(style.thumb_offset, 20.0);
}
