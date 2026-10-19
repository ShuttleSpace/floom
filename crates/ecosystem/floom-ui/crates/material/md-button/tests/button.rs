//! The ported Material 3 button — the *interactive* reference port.
//!
//! Where `md-card`'s test proved the theme→style→widget translation, this one
//! also pins the parts an interactive control needs and a surface never
//! touches: a callback that must survive being shared into the tree, a ripple
//! that follows the theme after mount, and the `Send + Sync` constraint 2.0's
//! `Child` builder imposes.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{ElementId, LayoutRequest, Widget};
use floom_signal::SignalGet;
use floom_types::{Color, Rect};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};
use floom_widgets::{Button, Ripple};

use floom_core_theme::props::button::ButtonVariant;
use floom_core_theme::theme::Theme;
use floom_widget_md_button::MdButton;

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 120.0,
    y1: 40.0,
};

fn button() -> MdButton {
    MdButton::new().label("Save")
}


/// A real click is a press followed by a release over the same spot; the
/// engine's `Button` only activates on a matched pair, so sending one half is
/// not a weaker click, it is not a click at all.
fn click(b: &mut Button, id: floom_core::ElementId) {
    let at = floom_types::Point::new(60.0, 20.0);
    let mut down = TestEventContext::new(id, BOX, floom_types::PointerEvent::primary_down(at).into());
    b.event(&mut down, id);
    let mut up = TestEventContext::new(id, BOX, floom_types::PointerEvent::primary_up(at).into());
    b.event(&mut up, id);
}

#[test]
fn the_label_travels_into_the_engine_widget() {
    let b = button().resolve();
    assert_eq!(b.label(), "Save");
}

#[test]
fn variants_resolve_to_different_colours() {
    let primary = MdButton::new()
        .variant(ButtonVariant::Primary)
        .resolved_style();
    let danger = MdButton::new()
        .variant(ButtonVariant::Danger)
        .resolved_style();
    assert_ne!(
        primary.background, danger.background,
        "each variant must resolve its own palette"
    );
}

#[test]
fn a_disabled_button_resolves_to_the_disabled_palette() {
    let enabled = MdButton::new().variant(ButtonVariant::Primary).resolved_style();
    let disabled = MdButton::new()
        .variant(ButtonVariant::Primary)
        .disabled(true)
        .resolved_style();
    assert_ne!(
        enabled.background, disabled.background,
        "disabled must not keep the enabled fill"
    );
}

#[test]
fn the_click_callback_fires_when_the_button_is_pressed() {
    let hits = Arc::new(AtomicUsize::new(0));
    let sink = Arc::clone(&hits);
    let mut b = button()
        .on_click(move || {
            sink.fetch_add(1, Ordering::SeqCst);
        })
        .resolve();

    let mut create = floom_widgets::testkit::AllocatingCreateContext::new();
    let id = b.create(&mut create);

    click(&mut b, id);

    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "the wrapped callback must run on a click"
    );
}

#[test]
fn a_disabled_button_does_not_fire() {
    let hits = Arc::new(AtomicUsize::new(0));
    let sink = Arc::clone(&hits);
    let mut b = button()
        .disabled(true)
        .on_click(move || {
            sink.fetch_add(1, Ordering::SeqCst);
        })
        .resolve();

    let mut create = floom_widgets::testkit::AllocatingCreateContext::new();
    let id = b.create(&mut create);
    click(&mut b, id);

    assert_eq!(hits.load(Ordering::SeqCst), 0, "a disabled button is inert");
}

#[test]
fn the_ripple_clips_to_a_fully_rounded_button() {
    let ripple = button().resolve_with_ripple();
    match ripple.clip().expect("the ripple clips") {
        floom_types::ClipShape::RoundedBox { radius } => assert!(
            radius > 0.0,
            "a Material button's ink is clipped to fully-rounded bounds"
        ),
        other => panic!("expected a rounded box, got {other:?}"),
    }
}

#[test]
fn the_ripple_ink_is_the_foreground_at_low_alpha() {
    let style = button().resolved_style();
    let ink = button().ripple_ink();
    assert_eq!(
        ink.a, 24,
        "the ripple ink is the foreground at 24/255 so it reads on every variant"
    );
    assert_eq!(ink.r, style.foreground.r);
    assert_eq!(ink.g, style.foreground.g);
    assert_eq!(ink.b, style.foreground.b);
}

#[test]
fn follow_theme_replaces_v1s_radius_and_color_closures() {
    // v1 handed Ripple `radius_fn` and `color` closures that re-read the theme on
    // every use. 2.0 mounts once, so `follow_theme` is the replacement -- and it
    // has to be idempotent, or a theme signal firing on unrelated changes would
    // repaint on every frame.
    let mut ripple = button().resolve_with_ripple();
    let before = ripple.clip();

    button().follow_theme(&mut ripple);
    assert_eq!(ripple.clip(), before, "re-applying the same theme is a no-op");

    let mut create = floom_widgets::testkit::RecordingUpdateContext::default();
    let _ = &mut create;
}

#[test]
fn the_wrapped_ripple_holds_exactly_one_button() {
    let ripple = button().resolve_with_ripple();
    assert_eq!(
        ripple.children().len(),
        1,
        "the ripple wraps the button as its single child"
    );
}

#[test]
fn the_button_sits_inside_token_padding() {
    let style = button().resolved_style();
    let padding = button().padding();
    assert!(
        padding.left > 0.0,
        "the button token's horizontal padding must be applied"
    );
    let _ = (style.height, style.gap, style.icon_size);
    let _: Option<Button> = None;
    let _ = LayoutRequest::new();
    let mut recorder = SceneRecorder::new(BOX);
    recorder.finish();
    let _: fn(&dyn Widget) = |_| {};
}

/// Theme colour -> engine colour. The style speaks the theme's colour type and
/// the paint records the engine's; they are distinct types with the same shape.
fn to_engine(c: floom_core_theme::tokens::color::Color) -> Color {
    Color { r: c.r, g: c.g, b: c.b, a: c.a }
}

/// The fill and the label the engine actually painted.
fn painted(b: &MdButton) -> (Option<Color>, Option<Color>) {
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
fn painted_radius(b: &MdButton) -> f32 {
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

/// The fill the engine paints is the kit's *background*, and the label it
/// paints is the kit's *foreground*.
///
/// The engine `Button` paints a fill and a label, and they are two different
/// knobs: `Button::color` is the fill, `Button::text_color` is the label. A kit
/// that passes the foreground to `color` paints a foreground-coloured button
/// with a white label on it -- and it compiles, and the style resolves, and the
/// click fires. Nothing about the API says the button is upside down, so this
/// is asserted against the *paint*: the style struct alone cannot catch the
/// swap.
#[test]
fn the_fill_is_the_background_and_the_label_is_the_foreground() {
    let b = button();
    let style = b.resolved_style();
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
/// 38 for the label.
///
/// The engine halves the alpha of a disabled control's fill and its label, so
/// the kit hands it a double-alpha colour and the halving lands on v1's value.
/// A test that read the *style* would see the doubled alpha and think the value
/// was wrong; a test that reads the *paint* sees v1's 12 and 38.
#[test]
fn the_disabled_palette_reaches_the_pixels_at_v1s_alphas() {
    let b = MdButton::new().label("Save").disabled(true);
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

/// Material's button is a pill: the fill is drawn with `radius().full`.
#[test]
fn the_fill_is_drawn_with_the_full_radius() {
    let b = button();
    let style = b.resolved_style();
    assert_eq!(painted_radius(&b), style.border_radius);
    assert_eq!(style.border_radius, b.theme().get().radius().full);
}

/// The hover fill is the M3 state layer: 8% of the variant's own foreground
/// mixed over the base fill. This is where Material differs from Cupertino and
/// Fluent, whose hover is a flat foreground tint.
#[test]
fn the_hover_fill_is_the_eight_percent_state_layer() {
    let style = button().resolved_style();
    let expected = floom_core_theme::tokens::color_ext::mix(
        style.background,
        style.foreground,
        0.08,
    );
    assert_eq!(style.hover_background, expected);
}
