use floem_core_theme::theme::{CupertinoTheme, Theme};
use floem_widget_cupertino_card::CupertinoCard;

#[test]
fn card_exposes_neutral_contract_api() {
    let card = CupertinoCard::new().elevation(3);

    assert_eq!(card.props().elevation, 3);
    assert_eq!(card.resolved_style().design_system, "cupertino");
}

#[test]
fn card_style_resolves_from_cupertino_theme() {
    let theme = CupertinoTheme::new();
    let style = CupertinoCard::new()
        .elevation(3)
        .with_theme(theme.clone())
        .resolved_style();

    assert_eq!(style.container_color, theme.colors().surface);
    assert_eq!(style.content_color, theme.colors().on_surface);
    assert_eq!(style.border_radius, 12.0);
    assert_eq!(style.shadow, theme.shadow().levels[3]);
}
