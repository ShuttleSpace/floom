//! Cupertino's button: the palette translation, and the fill/label pair the
//! engine button actually paints.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{ElementId, Widget};
use floom_types::Color;
use floom_types::Rect;
use floom_widgets::testkit::AllocatingCreateContext;
use floom_widgets::testkit::{PaintOp, SceneRecorder, TestEventContext};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::{set_design_system, DesignSystem};
use floom_core_theme::props::button::{ButtonSize, ButtonVariant};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color_ext::*;

use floom_widget_cupertino_button::{CupertinoButton, CupertinoButtonStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 120.0,
    y1: 40.0,
};

/// Selects Cupertino and hands back its theme.
fn theme() -> MultiTheme {
    set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn button() -> CupertinoButton {
    CupertinoButton::new().with_theme(theme()).label("Save")
}

fn style_of(b: &CupertinoButton) -> CupertinoButtonStyle {
    b.resolved_style()
}

/// Theme colour -> engine colour. The style speaks the theme's colour type and
/// the paint records the engine's; they are distinct types with the same shape.
fn to_engine(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// The fill and the label the engine actually painted.
fn painted(b: &CupertinoButton) -> (Option<Color>, Option<Color>) {
    let mut b = b.resolve();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = b.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    b.paint(&mut recorder, id);
    recorder.finish();
    let ops = recorder.ops();

    let fill = ops.iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    let label = ops.iter().find_map(|op| match op {
        PaintOp::Text { color, .. } => Some(*color),
        _ => None,
    });
    (fill, label)
}

/// The radius the fill was painted with.
fn painted_radius(b: &CupertinoButton) -> f32 {
    let mut b = b.resolve();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = b.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    b.paint(&mut recorder, id);
    recorder.finish();
    recorder
        .ops()
        .iter()
        .find_map(|op| match op {
            PaintOp::RoundedRect { radius, .. } => Some(*radius),
            _ => None,
        })
        .expect("the button paints a rounded fill")
}

// --- the fill/label pair ---------------------------------------------------

/// The fill the engine paints is the kit's *background*, and the label it
/// paints is the kit's *foreground*. The two are different colours, so a swap
/// is visible.
#[test]
fn the_fill_is_the_background_and_the_label_is_the_foreground() {
    let b = button();
    let style = style_of(&b);
    let (fill, label) = painted(&b);
    assert_eq!(
        fill,
        Some(to_engine(style.background)),
        "the fill must be the background"
    );
    assert_eq!(
        label,
        Some(to_engine(style.foreground)),
        "the label must be the foreground"
    );
    assert_ne!(
        style.background, style.foreground,
        "a primary button's fill and label must differ, or the swap is invisible"
    );
}

/// The disabled palette reaches the pixels at v1's alphas: 12 for the fill and
/// 38 for the label. The kit hands the engine double-alpha colours and the
/// engine halves them; this asserts the result of that round trip.
#[test]
fn the_disabled_palette_reaches_the_pixels_at_v1s_alphas() {
    let b = CupertinoButton::new()
        .with_theme(theme())
        .label("Save")
        .disabled(true);
    let (fill, label) = painted(&b);
    assert_eq!(
        fill.map(|c| c.a),
        Some(12),
        "the disabled fill must land on v1's 12/255"
    );
    assert_eq!(
        label.map(|c| c.a),
        Some(38),
        "the disabled label must land on v1's 38/255"
    );
}

// --- the numbers that are this kit's own ------------------------------------

/// The corner radius the fill is drawn with: lg.
#[test]
fn the_fill_is_drawn_with_lg_radius() {
    let b = button();
    let style = style_of(&b);
    assert_eq!(painted_radius(&b), style.border_radius);
    assert_eq!(style.border_radius, theme().radius().lg);
}

/// The size decides the height: the token height, 8 shorter for a small
/// button and 8 taller for a large one.
#[test]
fn the_size_decides_the_height() {
    let tokens = match theme().component(ComponentId::Button) {
        ComponentTokens::Button(t) => t,
        _ => unreachable!(),
    };
    let small = CupertinoButton::new()
        .with_theme(theme())
        .size(ButtonSize::Small)
        .resolved_style();
    let medium = CupertinoButton::new()
        .with_theme(theme())
        .size(ButtonSize::Medium)
        .resolved_style();
    let large = CupertinoButton::new()
        .with_theme(theme())
        .size(ButtonSize::Large)
        .resolved_style();
    assert_eq!(small.height, tokens.height - 8.0);
    assert_eq!(medium.height, tokens.height);
    assert_eq!(large.height, tokens.height + 8.0);
}

/// Each variant resolves its own palette.
#[test]
fn the_variants_resolve_their_own_palette() {
    let primary = CupertinoButton::new()
        .with_theme(theme())
        .variant(ButtonVariant::Primary)
        .resolved_style();
    let secondary = CupertinoButton::new()
        .with_theme(theme())
        .variant(ButtonVariant::Secondary)
        .resolved_style();
    let danger = CupertinoButton::new()
        .with_theme(theme())
        .variant(ButtonVariant::Danger)
        .resolved_style();
    assert_ne!(primary.background, secondary.background);
    assert_ne!(primary.background, danger.background);
}

/// The ghost and text variants have no fill at all -- a transparent one -- and a
/// `primary` label.
#[test]
fn the_ghost_variant_has_no_fill() {
    let ghost = CupertinoButton::new()
        .with_theme(theme())
        .variant(ButtonVariant::Ghost)
        .resolved_style();
    assert_eq!(
        ghost.background.a, 0,
        "a ghost button has a transparent fill"
    );
    assert_eq!(ghost.foreground, theme().colors().primary);
}

/// The disabled palette is `on_surface` at 12/255 and 38/255.
#[test]
fn the_disabled_palette_is_on_surface_at_twelve_and_thirty_eight() {
    let disabled = CupertinoButton::new()
        .with_theme(theme())
        .disabled(true)
        .resolved_style();
    assert_eq!(
        disabled.background,
        with_alpha(theme().colors().on_surface, 12)
    );
    assert_eq!(
        disabled.foreground,
        with_alpha(theme().colors().on_surface, 38)
    );
}

/// The hover fill is the foreground at 12/255 -- a flat tint, not a mix over
/// the base. This is where Cupertino and Fluent differ from Material, whose
/// hover is a state-layer mix.
#[test]
fn the_hover_fill_is_the_foreground_at_twelve() {
    let style = button().resolved_style();
    let expected = with_alpha(style.foreground, 12);
    assert_eq!(style.hover_background, expected);
}

/// The press fill is the foreground at 20/255.
#[test]
fn the_press_fill_is_the_foreground_at_twenty() {
    let style = button().resolved_style();
    let expected = with_alpha(style.foreground, 20);
    assert_eq!(style.press_background, expected);
}

/// A disabled button has no hover of its own: v1's state layer returned the
/// disabled fill, and the engine ignores the interaction colours while a
/// control is disabled.
#[test]
fn a_disabled_button_has_no_hover_of_its_own() {
    let disabled = CupertinoButton::new()
        .with_theme(theme())
        .disabled(true)
        .resolved_style();
    assert_eq!(disabled.hover_background, disabled.background);
    assert_eq!(disabled.press_background, disabled.background);
}

/// The click callback fires when the button is pressed.
#[test]
fn the_click_callback_fires() {
    let hits = Arc::new(AtomicUsize::new(0));
    let sink = Arc::clone(&hits);
    let mut b = CupertinoButton::new()
        .with_theme(theme())
        .label("Save")
        .on_click(move || {
            sink.fetch_add(1, Ordering::SeqCst);
        })
        .resolve();

    let mut create = AllocatingCreateContext::new();
    let id = b.create(&mut create);

    let at = floom_types::Point::new(60.0, 20.0);
    let mut down =
        TestEventContext::new(id, BOX, floom_types::PointerEvent::primary_down(at).into());
    b.event(&mut down, id);
    let mut up = TestEventContext::new(id, BOX, floom_types::PointerEvent::primary_up(at).into());
    b.event(&mut up, id);

    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "the callback must run on a click"
    );
}

/// A disabled button does not fire.
#[test]
fn a_disabled_button_does_not_fire() {
    let hits = Arc::new(AtomicUsize::new(0));
    let sink = Arc::clone(&hits);
    let mut b = CupertinoButton::new()
        .with_theme(theme())
        .label("Save")
        .disabled(true)
        .on_click(move || {
            sink.fetch_add(1, Ordering::SeqCst);
        })
        .resolve();

    let mut create = AllocatingCreateContext::new();
    let id = b.create(&mut create);

    let at = floom_types::Point::new(60.0, 20.0);
    let mut down =
        TestEventContext::new(id, BOX, floom_types::PointerEvent::primary_down(at).into());
    b.event(&mut down, id);
    let mut up = TestEventContext::new(id, BOX, floom_types::PointerEvent::primary_up(at).into());
    b.event(&mut up, id);

    assert_eq!(hits.load(Ordering::SeqCst), 0, "a disabled button is inert");
}
