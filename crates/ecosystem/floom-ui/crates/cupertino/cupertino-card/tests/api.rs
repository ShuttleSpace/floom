use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_cupertino_card::CupertinoCard;

#[test]
fn card_exposes_neutral_contract_api() {
    let card = CupertinoCard::new().elevation(3);

    assert_eq!(card.props().elevation, 3);
    assert_eq!(card.resolved_style().design_system, "cupertino");
}

/// This test asserted `style.border_radius == 12.0`, and it **passed** — which is
/// worse than failing.
///
/// Cupertino's card token is `10.0`. `12.0` is Material's. The assertion only held
/// because `MultiTheme::new_for(Cupertino)` does not select Cupertino: it reads a
/// process-global design system that nothing in this workspace ever sets, so the
/// "Cupertino theme" was Material's and the number matched by coincidence. The
/// sibling test in `fluent-card` asserted `4.0` and failed, because `4.0` is not
/// anybody's.
///
/// So the two inherited card tests disagreed about what their own kit's radius was,
/// and the one that was wrong in the direction of the default passed. Asserted
/// against the token instead of a literal, which is what the test's name claims: the
/// point is that the style *resolves from* the theme.
#[test]
fn card_style_resolves_from_cupertino_theme() {
    let theme = MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino);
    let style = CupertinoCard::new()
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
