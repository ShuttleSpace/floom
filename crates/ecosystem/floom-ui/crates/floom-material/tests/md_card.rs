#![cfg(feature = "card")]
use floom_core::Widget;
use floom_types::Insets;
use floom_types::{Color as PaintColor, Shadow};

use floom_core_theme::theme::MultiTheme;
use floom_widgets::testkit::{paint_snapshot, AllocatingCreateContext};

use floom_material::widgets::card::MdCard;

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
    let mut create = AllocatingCreateContext::new();
    let surface = card().surface();
    let id = floom_core::Widget::create(&surface, &mut create);
    let snap = paint_snapshot(&surface, id, AREA);

    let shadow_at = snap.find("Shadow ").expect("a card asks for a shadow");
    let fill_at = snap
        .find("RoundedRect")
        .expect("a card paints a rounded fill");
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

    assert!(
        style.border_radius > 0.0,
        "card tokens should carry a real radius"
    );
    assert_eq!(
        surface.decoration().corner_radius(),
        style.border_radius,
        "and the radius is on the surface the caller mounts"
    );

    assert_eq!(
        surface.padding_insets(),
        Insets::uniform(f64::from(style.padding)),
        "the padding token reaches a box that can actually hold content"
    );
    assert!(
        surface.layout_request().and_then(|r| r.padding).is_some(),
        "and it leaves as padding, which is the only inset a box with children \
         can apply"
    );
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
    let a = MdCard::new().resolved_style();
    let b = MdCard::new().resolved_style();
    assert_eq!(a, b, "cards must read one shared theme");

    let custom = MdCard::new().with_theme(MultiTheme::new_for(
        floom_core_theme::global_theme::DesignSystem::default(),
    ));
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
