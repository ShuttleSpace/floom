use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_fluent_card::FluentCard;

#[test]
fn card_exposes_neutral_contract_api() {
    let card = FluentCard::new().elevation(3);

    assert_eq!(card.props().elevation, 3);
    assert_eq!(card.resolved_style().design_system, "fluent");
}

/// This test used to assert `style.border_radius == 4.0`, a literal that matches
/// no design system in the table: `CardTokens::from_spacing` — which is what
/// Fluent gets — says `12.0`, and Material says `10.0`.
///
/// It was also never run: the crate did not compile (its `lib.rs` still called
/// `Ripple::new(inner)` and `.radius_fn(..)`, both gone), so the assertion sat
/// behind a build error and the crate's test suite had never been executed.
///
/// Asserted against the token instead of a literal, which is what the test's own
/// name claims: the point is that the style *resolves from* the theme, so a change
/// to the token table should move this number, not break it.
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
