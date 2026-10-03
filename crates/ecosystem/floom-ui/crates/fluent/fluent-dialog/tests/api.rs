use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_fluent_dialog::FluentDialog;

#[test]
fn dialog_exposes_neutral_contract_api() {
    let dialog = FluentDialog::new()
        .title("Confirm")
        .content("Continue?")
        .open(true);

    assert_eq!(dialog.props().title, "Confirm");
    assert_eq!(dialog.props().content, "Continue?");
    assert!(dialog.props().open);
    assert_eq!(dialog.resolved_style().design_system, "fluent");
}

#[test]
fn dialog_style_resolves_from_fluent_theme() {
    let theme = MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent);
    let style = FluentDialog::new()
        .with_theme(theme.clone())
        .resolved_style();

    assert_eq!(style.scrim_color, theme.colors().scrim);
    assert_eq!(style.container_color, theme.colors().surface);
    assert_eq!(style.content_color, theme.colors().on_surface);
    assert_eq!(style.max_width, 540.0);
    assert_eq!(style.border_radius, 8.0);
}
