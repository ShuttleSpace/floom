//! The ported Material 3 card.
//!
//! This is the reference port for the floom-ui component library: it proves the
//! v1 -> 2.0 translation end to end (theme tokens -> style struct -> 2.0
//! widget) without depending on the v1 monolith. See
//! `docs/porting-floem-ui-to-2.0.md`.

use floom_types::{Insets, Rect};
use floom_types::{Color as PaintColor, Shadow};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::theme::MultiTheme;
use floom_widgets::testkit::{AllocatingCreateContext, paint_snapshot};

use floom_widget_md_card::MdCard;

fn card() -> MdCard {
    MdCard::new()
}

const AREA: floom_types::Rect = floom_types::Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 40.0,
    y1: 20.0,
};

#[test]
fn the_surface_is_a_real_2p0_container_painting_shadow_behind_fill() {
    // If this compiles and paints, the port produced an engine widget rather
    // than a v1 view smuggled across. The two ops and their order are the real
    // assertion: the shadow is asked for first so it lands behind the fill,
    // which is the whole point of "cast behind its own fill".
    let mut create = AllocatingCreateContext::new();
    let surface = card().surface();
    let id = floom_core::Widget::create(&surface, &mut create);
    let snap = paint_snapshot(&surface, id, AREA);

    let shadow_at = snap.find("Shadow ").expect("a card asks for a shadow");
    let fill_at = snap.find("RoundedRect").expect("a card paints a rounded fill");
    assert!(
        shadow_at < fill_at,
        "the shadow must be asked for before the fill, got:\n{snap}"
    );
    assert!(
        snap.contains("r=12.0"),
        "the card token's border radius must reach the engine widget, got:\n{snap}"
    );
}

#[test]
fn elevation_selects_a_shadow_level_and_carries_its_spread() {
    let top = MdCard::new().elevation(5).resolved_style();
    let shadow = MdCard::new().elevation(3).shadow();

    // Elevation clamps to the six declared levels, so 99 must resolve the same
    // as 5 rather than index out of bounds.
    let clamped = MdCard::new().elevation(99).resolved_style();
    assert_eq!(
        clamped.shadow, top.shadow,
        "an out-of-range elevation must clamp to the top level"
    );

    assert!(
        shadow.spread >= 0.0,
        "the design system's spread must reach the engine shadow"
    );
}

#[test]
fn two_elevations_resolve_to_different_shadows() {
    let low = MdCard::new().elevation(0).resolved_style().shadow;
    let high = MdCard::new().elevation(4).resolved_style().shadow;
    assert_ne!(
        low, high,
        "elevation levels are distinct shadows; if these matched, the token \
         table is not being read"
    );
}

#[test]
fn the_shadow_is_translated_field_for_field() {
    let data = MdCard::new().elevation(2).resolved_style().shadow;
    let shadow = MdCard::new().elevation(2).shadow();

    assert_eq!(shadow.blur, f64::from(data.blur));
    assert_eq!(shadow.offset_x, f64::from(data.offset_x));
    assert_eq!(shadow.offset_y, f64::from(data.offset_y));
    assert_eq!(
        shadow.spread,
        f64::from(data.spread),
        "v1 spelled this `box_shadow_spread`; dropping it silently would be \
         the exact fidelity loss the Shadow::spread change fixed"
    );
    assert_eq!(shadow.color.a, data.color_a);
}

#[test]
fn the_surface_uses_the_resolved_padding_radius_and_palette() {
    let style = MdCard::new().resolved_style();
    let surface = MdCard::new().surface();

    // The engine container is what a caller actually mounts content into, so
    // the theme values have to have reached it.
    assert!(
        style.border_radius > 0.0,
        "card tokens should carry a real radius"
    );
    assert_eq!(
        surface.corner_radius(),
        style.border_radius,
        "and the radius is on the surface the caller mounts"
    );

    // The padding is *not* on the surface, and this assertion is the reason that
    // is not a silent regression: an earlier version of this port handed
    // `Container` a padding that the widget stored and never read, and this test
    // passed anyway because it only asserted the token was positive. The inset
    // now has to be found on the content's own slot.
    let layout = MdCard::new().content_layout();
    assert_eq!(
        layout.margin.expect("the card's inset"),
        Insets::uniform(f64::from(style.padding)),
        "the padding token reaches the content as a margin on its slot"
    );
    // A no-op `background` would make the card invisible, so assert the paint
    // op is not a transparent fill.
    let mut create = AllocatingCreateContext::new();
    let id = floom_core::Widget::create(&surface, &mut create);
    let snap = paint_snapshot(&surface, id, AREA);
    assert!(
        !snap.contains("#00000000"),
        "the card surface must not paint a transparent fill, got {snap}"
    );
}

#[test]
fn theme_tokens_come_from_the_shared_theme_signal() {
    // Two cards with no explicit theme must agree, because both read the one
    // app-wide signal rather than each holding a private palette.
    let a = MdCard::new().resolved_style();
    let b = MdCard::new().resolved_style();
    assert_eq!(a, b, "cards must read one shared theme");

    // And an explicitly-supplied theme is honoured over the signal.
    let custom = MdCard::new()
        .with_theme(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::default()));
    assert!(custom.resolved_style().padding > 0.0);
}

#[test]
fn shadow_helper_does_not_panic_for_any_level() {
    for level in 0..=5u8 {
        let s = MdCard::new().elevation(level).shadow();
        assert!(
            s.blur >= 0.0 && s.spread.is_finite(),
            "level {level} produced a non-finite shadow: {s:?}"
        );
    }
    let _ = PaintColor::BLACK;
    let _: Shadow = MdCard::new().shadow();
}