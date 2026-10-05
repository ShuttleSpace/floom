//! The ported Fluent card.
//!
//! The second card in the library, and the one that proves a *ported* shape had to
//! grow: `md-card` is a decorated box and this one is a decorated box **inside a
//! press ripple**. So this suite is as much about the composition — surface, then
//! ripple over it, with the content inside the surface and not beside it — as about
//! the tokens.

use floom_core::{Widget, WidgetAny};
use floom_types::{Insets, Rect};

use floom_core_theme::theme::MultiTheme;
use floom_widgets::testkit::{AllocatingCreateContext, paint_snapshot};

use floom_widget_fluent_card::FluentCard;

const CARD: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 120.0,
};

fn card() -> FluentCard {
    FluentCard::new()
}

/// The widget the ripple wraps, still type-erased.
///
/// `Box<dyn WidgetAny>` does not deref to `Widget`, so a caller holding one has
/// to downcast before it can read anything off it. Every assertion below goes
/// through here, which is also what keeps the downcast honest: one place, and it
/// names the expected type.
fn inside(ripple: &floom_widgets::Ripple) -> Box<dyn WidgetAny> {
    ripple.children()[0].build()
}

/// The ripple's child, recovered as the card's [`Surface`].
///
/// The downcast is the assertion: a `Container`-based port, or a ripple that had
/// replaced the surface rather than wrapped it, would fail right here.
fn surface_inside(ripple: &floom_widgets::Ripple) -> floom_widgets::Surface {
    inside(ripple)
        .as_any()
        .downcast_ref::<floom_widgets::Surface>()
        .expect("the ripple wraps the card's Surface, and nothing else")
        .clone()
}

// --- the translation ------------------------------------------------------

/// Fluent's own theme instance, so the card resolves against its design system
/// rather than whatever a caller left in the signal.
#[test]
fn a_new_card_resolves_fluent_tokens() {
    assert_eq!(
        card().resolved_style().design_system,
        "fluent",
        "the design system is Fluent, which is what `new()` seeds the signal with"
    );
}

#[test]
fn the_tokens_reach_the_engine_surface() {
    let resolved = card().resolved_style();
    let surface = card().surface();
    let declared = surface
        .layout_request()
        .expect("a padded surface has a request");

    assert_eq!(
        declared.padding,
        Some(Insets::uniform(f64::from(resolved.padding))),
        "the token's padding, inside the box"
    );
    assert_eq!(
        surface.decoration().corner_radius(),
        card().corner_radius(),
        "and the theme's radius scale — v1 drew `t.radius().xl`, not the token"
    );
    assert!(
        surface.decoration().drop_shadow().is_some(),
        "elevation reaches the engine as a drop shadow"
    );
}

#[test]
fn the_ink_is_offered_rather_than_baked_in() {
    let resolved = card().resolved_style();
    assert_eq!(
        card().content_color().r,
        resolved.content_color.r,
        "v1 set this on the box, which has nowhere to live in 2.0: a Surface \
         paints a fill and only the child knows it is text"
    );
}

// --- the corner radius, and the difference the port had to keep ------------

/// v1 read `CardTokens::border_radius` into its style struct and never used it,
/// while drawing the box at `t.radius().xl`. Keeping both apart is what makes the
/// translation honest — and for Fluent the two differ, which is how the
/// difference was noticed at all.
#[test]
fn the_drawn_radius_is_the_themes_not_the_cards_token() {
    let token_radius = card().resolved_style().border_radius;
    let drawn_radius = card().corner_radius();
    assert!(
        (token_radius - drawn_radius).abs() > f32::EPSILON,
        "Fluent's two radii differ: token {token_radius}, drawn {drawn_radius}"
    );
    assert_eq!(
        card().surface().decoration().corner_radius(),
        drawn_radius,
        "and the box uses the one v1 drew"
    );
}

// --- the ripple -----------------------------------------------------------

/// The whole difference from `md-card`: a card you can press is still a card that
/// holds its content, so the ripple goes *around* the surface rather than
/// replacing it.
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

/// The control for the composition: the surface inside the ripple really does
/// paint. If the ripple had replaced the surface, this would be empty.
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

// --- the content ----------------------------------------------------------

/// The assertion that distinguishes a surface from a rectangle, and that a
/// `Container`-based port would have failed: the content goes *inside* the box,
/// under the ripple.
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

/// A 2.0 slot is a replayed `Fn`, so the surface is rebuilt on every rebuild
/// rather than captured. This is the test that holds that design honest: two
/// replays of the same ripple must both produce a surface *with the content*.
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
    assert!(
        surface.decoration().drop_shadow().is_some(),
        "elevation"
    );
}

// --- elevation ------------------------------------------------------------

/// Elevation clamps to the declared levels, so 99 must resolve the same as 5
/// rather than index out of bounds — v1 clamped before indexing too.
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