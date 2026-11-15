#![cfg(feature = "card")]
//! The ported Fluent card.
//!

use floom_core::{Widget, WidgetAny};
use floom_types::Rect;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widgets::testkit::{paint_snapshot, AllocatingCreateContext};

use floom_cupertino::widgets::card::CupertinoCard;

const CARD: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 120.0,
};

/// Activate `ds`, then build the card against it.
///
/// One helper, and it takes the design system as an argument on purpose. The first
/// version had two — a bare `card()` plus a separate `cupertino()` — and `card()`
/// re-activated the kit on every call, so the control test that switched the global
/// to Material to prove a number was *read* rather than hard-coded had its switch
/// undone one line later and failed against a correct implementation.
///
/// `floom_core_theme`'s design-system signal is a `thread_local`, and the test
/// harness gives each test its own thread, so this cannot leak into a sibling.
fn card_under(ds: DesignSystem) -> CupertinoCard {
    floom_core_theme::global_theme::set_design_system(ds);
    CupertinoCard::new()
}

/// The card, resolving against this kit's own tokens.
fn card() -> CupertinoCard {
    card_under(DesignSystem::Cupertino)
}

/// The active kit's `spacing().md`, read through the same global the card reads.
fn spacing_md(ds: DesignSystem) -> f32 {
    floom_core_theme::global_theme::set_design_system(ds);
    MultiTheme::new_for(ds).spacing().md
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

/// The style names its design system, and — this is the part worth asserting —
/// the tokens behind it really are Fluent's.
///
/// The first version of this test asserted only `design_system == "fluent"`, which
/// is a string literal in the crate. It passed while the card was resolving
/// Material's numbers, which is what made the mistake worth recording.
#[test]
fn a_new_card_resolves_its_own_kits_tokens() {
    assert_eq!(
        card().resolved_style().design_system,
        "cupertino",
        "the crate names its design system"
    );

    let style = card().resolved_style();
    assert_eq!(
        style.border_radius, 10.0,
        "Cupertino's card token is 10.0. Material's is 12.0 and Fluent's 8.0, so \
         this is the assertion that the numbers came from Cupertino at all"
    );
    assert_eq!(
        style.padding,
        spacing_md(DesignSystem::Cupertino),
        "and the padding comes from Cupertino's spacing scale"
    );
}

/// A pin, not a test of the card: `MultiTheme::new_for(ds)` does not select `ds`,
/// and a `MultiTheme` is not a snapshot of any one design system.
///
/// `activate` is a documented no-op placeholder. `MultiTheme` holds all three
/// themes and dispatches on `current_design_system()` — a process-global — at
/// **every read**. So a `MultiTheme` built "for Cupertino" answers with **Material**
/// unless something set the global first, and nothing in this 299-crate workspace
/// ever does.
///
/// The ordering below is load-bearing and was got wrong first: reading `named_cupertino`
/// *after* switching the global yields 10.0, because the read is live. That is the
/// second half of the finding, not a detail.
///
/// Delete this test when the global stops being the selector; it is here so the day
/// that changes, the change is loud. It is also why every other test in this file
/// activates its design system explicitly — without that, all of them would be
/// measuring Material and calling it Cupertino.
#[test]
fn new_for_does_not_select_the_design_system_it_is_given() {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    let named_cupertino = MultiTheme::new_for(DesignSystem::Cupertino);
    assert_eq!(
        card_radius(&named_cupertino),
        12.0,
        "CONTROL: the one *named* Cupertino read Material's token, because `new_for` \
         ignored its argument"
    );

    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    let really_cupertino = MultiTheme::new_for(DesignSystem::Cupertino);
    assert_eq!(
        card_radius(&really_cupertino),
        10.0,
        "and only once the global is set does the same constructor answer with Cupertino"
    );
}

/// The `border_radius` a `CardTokens` resolves to — read through the same path the
/// card reads.
fn card_radius(theme: &MultiTheme) -> f32 {
    match theme.component(ComponentId::Card) {
        ComponentTokens::Card(tokens) => tokens.border_radius,
        _ => unreachable!("ComponentId::Card resolves to CardTokens"),
    }
}

// --- the corner radius, and the difference the port had to keep ------------

/// v1 read `CardTokens::border_radius` into its style struct and never used it,
/// while drawing the box at `t.radius().xl`. The port keeps both, and the box uses
/// the one v1 drew.
///
/// **For Cupertino the two coincide** — token `10.0`, drawn `radius().md` `10.0` —
/// so this test asserts equality rather than difference. Cupertino is the *second*
/// kit whose first version of this assertion was wrong: `fluent-card`'s asserted
/// they differed and passed only because it was reading Material (`12.0`/`24.0`).
/// Material is the one kit where the discrepancy is real; Cupertino
/// (`10.0`/`10.0`) and Fluent (`8.0`/`8.0`) are not. That is why the port keeps
/// them apart as separate methods instead of collapsing them — one shape serves
/// all three kits, and only one of them needs the distinction.
#[test]
fn the_drawn_radius_is_the_themes_not_the_cards_token() {
    let token_radius = card().resolved_style().border_radius;
    let drawn_radius = card().corner_radius();
    assert_eq!(
        token_radius, drawn_radius,
        "Cupertino's card token and `radius().md` are both 10.0 — they coincide, \
         which is the opposite of what this test first claimed"
    );
    assert_eq!(
        card().surface().decoration().corner_radius(),
        drawn_radius,
        "and the box uses the one v1 drew, which is also the token's value"
    );
}

/// The control, and the mutation this whole pair of tests exists for.
///
/// Cupertino's card token and Cupertino's `radius().md` are both `10.0`, so every
/// assertion above would also pass against a hard-coded `10.0`. Switching the
/// design system is what separates the two: Material's `radius().md` is `12.0`.
///
/// If someone "simplifies" `corner_radius()` into a literal, this goes red. That is
/// the point — the three kits reach their radius by three *different* expressions
/// (Material reads the token, Cupertino reads `radius().md`, Fluent reads
/// `radius().xl`) and only the numbers happen to agree today. An earlier version of
/// this file claimed Material was the kit where the token and the drawn radius
/// disagreed; it does not, and the claim came from reading Material's numbers while
/// believing they were Cupertino's.
#[test]
fn the_drawn_radius_follows_the_theme_rather_than_a_literal() {
    assert_eq!(
        card().corner_radius(),
        10.0,
        "CONTROL: Cupertino's `radius().md`"
    );

    assert_eq!(
        card_under(DesignSystem::Material).corner_radius(),
        12.0,
        "and Material's `radius().md` is 12.0 — so the 10.0 above was read from \
         the theme, not baked in. A literal would fail here."
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
    assert!(surface.decoration().drop_shadow().is_some(), "elevation");
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
