#![cfg(feature = "card")]
//! The ported Fluent card.

use floom_core::{Widget, WidgetAny};
use floom_types::Rect;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widgets::testkit::{paint_snapshot, AllocatingCreateContext};

use floom_fluent::widgets::card::FluentCard;

const CARD: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 120.0,
};

fn card_under(ds: DesignSystem) -> FluentCard {
    floom_core_theme::global_theme::set_design_system(ds);
    FluentCard::new()
}

fn card() -> FluentCard {
    card_under(DesignSystem::Fluent)
}

fn spacing_md(ds: DesignSystem) -> f32 {
    floom_core_theme::global_theme::set_design_system(ds);
    MultiTheme::new_for(ds).spacing().md
}

fn inside(ripple: &floom_widgets::Ripple) -> Box<dyn WidgetAny> {
    ripple.children()[0].build()
}

fn surface_inside(ripple: &floom_widgets::Ripple) -> floom_widgets::Surface {
    inside(ripple)
        .as_any()
        .downcast_ref::<floom_widgets::Surface>()
        .expect("the ripple wraps the card's Surface, and nothing else")
        .clone()
}

#[test]
fn a_new_card_resolves_its_own_kits_tokens() {
    assert_eq!(
        card().resolved_style().design_system,
        "fluent",
        "the crate names its design system"
    );

    let style = card().resolved_style();
    assert_eq!(
        style.border_radius, 8.0,
        "Fluent's card token is 8.0. Material's is 12.0 and Cupertino's 10.0, so \
         this is the assertion that the numbers came from Fluent at all"
    );
    assert_eq!(
        style.padding,
        spacing_md(DesignSystem::Fluent),
        "and the padding comes from Fluent's spacing scale"
    );
}

#[test]
fn new_for_does_not_select_the_design_system_it_is_given() {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    let named_fluent = MultiTheme::new_for(DesignSystem::Fluent);
    assert_eq!(
        card_radius(&named_fluent),
        12.0,
        "CONTROL: the one *named* Fluent read Material's token, because `new_for` \
         ignored its argument"
    );

    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    let really_fluent = MultiTheme::new_for(DesignSystem::Fluent);
    assert_eq!(
        card_radius(&really_fluent),
        8.0,
        "and only once the global is set does the same constructor answer with Fluent"
    );
}

fn card_radius(theme: &MultiTheme) -> f32 {
    match theme.component(ComponentId::Card) {
        ComponentTokens::Card(tokens) => tokens.border_radius,
        _ => unreachable!("ComponentId::Card resolves to CardTokens"),
    }
}

#[test]
fn the_drawn_radius_is_the_themes_not_the_cards_token() {
    let token_radius = card().resolved_style().border_radius;
    let drawn_radius = card().corner_radius();
    assert_eq!(
        token_radius, drawn_radius,
        "Fluent's card token and `radius().xl` are both 8.0 — they coincide, \
         which is the opposite of what this test first claimed"
    );
    assert_eq!(
        card().surface().decoration().corner_radius(),
        drawn_radius,
        "and the box uses the one v1 drew, which is also the token's value"
    );
}

#[test]
fn the_drawn_radius_follows_the_theme_rather_than_a_literal() {
    assert_eq!(
        card().corner_radius(),
        8.0,
        "CONTROL: Fluent's `radius().xl`"
    );

    assert_eq!(
        card_under(DesignSystem::Material).corner_radius(),
        24.0,
        "and Material's `radius().xl` is 24.0 — so the 8.0 above was read from \
         the theme, not baked in. A literal would fail here."
    );
}

#[test]
fn the_card_is_a_ripple_over_a_surface() {
    let ripple = card().view();
    assert_eq!(
        ripple.children().len(),
        1,
        "the ripple holds exactly one child"
    );
    surface_inside(&ripple);
}

#[test]
fn the_ripple_is_clipped_to_the_cards_own_radius() {
    let radius = card().corner_radius();
    let ripple = card().view();
    assert_eq!(
        ripple.clip_radius(),
        f64::from(radius),
        "v1 wrote `radius_fn(|t| t.radius().xl)`; resolved once here, same value"
    );
}

#[test]
fn the_surface_inside_the_ripple_paints() {
    let ripple = card().view();
    let surface = surface_inside(&ripple);
    let id = surface.create(&mut AllocatingCreateContext::new());
    let painted = paint_snapshot(&surface, id, CARD);
    assert!(!painted.is_empty(), "the surface paints: {painted}");
    assert!(
        painted.contains("RoundedRect"),
        "a filled, rounded box: {painted}"
    );
    assert!(
        painted.contains(&format!("r={}", card().corner_radius())),
        "drawn at the theme's radius: {painted}"
    );
}

#[test]
fn the_content_goes_inside_the_box() {
    assert_eq!(
        card().surface().children().len(),
        0,
        "no content, no children"
    );

    let ripple = card()
        .child(|| Box::new(floom_widgets::Text::new("card")))
        .view();
    assert_eq!(
        surface_inside(&ripple).children().len(),
        1,
        "and one when the caller supplied some — inside the box, not beside it"
    );
}

#[test]
fn the_slot_replays_rather_than_hands_out_one_widget() {
    let ripple = card()
        .child(|| Box::new(floom_widgets::Text::new("card")))
        .view();
    for round in 0..2 {
        let surface = inside(&ripple);
        assert_eq!(
            surface
                .as_any()
                .downcast_ref::<floom_widgets::Surface>()
                .expect("still the Surface on a replay")
                .children()
                .len(),
            1,
            "round {round}: a captured widget would hand out the same instance, \
             and the second replay would find its content already moved"
        );
    }
}

#[test]
fn the_padding_and_the_shadow_survive_the_ripple() {
    let ripple = card()
        .child(|| Box::new(floom_widgets::Text::new("x")))
        .view();
    let surface = surface_inside(&ripple);
    let declared = surface.layout_request().expect("a request");
    assert!(declared.padding.is_some(), "padding");
    assert!(surface.decoration().drop_shadow().is_some(), "elevation");
}

#[test]
fn elevation_is_clamped_to_the_declared_levels() {
    assert_eq!(
        card().elevation(99).props().elevation,
        5,
        "v1 clamped to 5 before indexing the levels"
    );
    assert_eq!(
        card().elevation(5).resolved_style().shadow,
        card().elevation(99).resolved_style().shadow,
        "so 5 and 99 are the same card"
    );
}

#[test]
fn two_elevations_resolve_to_different_shadows() {
    let low = card().elevation(0).resolved_style().shadow;
    let high = card().elevation(4).resolved_style().shadow;
    assert_ne!(
        low, high,
        "elevation levels are distinct shadows; if these matched, the token \
         table would be carrying one shadow for every level"
    );
}
