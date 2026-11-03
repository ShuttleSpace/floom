use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_fluent_card::FluentCard;

#[test]
fn card_exposes_neutral_contract_api() {
    let card = FluentCard::new().elevation(3);

    assert_eq!(card.props().elevation, 3);
    assert_eq!(card.resolved_style().design_system, "fluent");
}

#[test]
fn card_style_resolves_from_fluent_theme() {
    let theme = MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent);
    let style = FluentCard::new()
        .elevation(3)
        .with_theme(theme.clone())
        .resolved_style();

    assert_eq!(style.container_color, theme.colors().surface);
    assert_eq!(style.content_color, theme.colors().on_surface);
    assert_eq!(
        style.border_radius,
        match theme.component(floom_core_theme::component_tokens::ComponentId::Card) {
            floom_core_theme::component_tokens::ComponentTokens::Card(tokens) => {
                tokens.border_radius
            }
            _ => unreachable!("ComponentId::Card resolves to CardTokens"),
        },
        "the token's radius, whatever the token table currently says"
    );
    assert_eq!(style.shadow, theme.shadow().levels[3]);
}
