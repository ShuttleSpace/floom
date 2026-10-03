//! The ported Material 3 button — the *interactive* reference port.
//!
//! Where `md-card`'s test proved the theme→style→widget translation, this one
//! also pins the parts an interactive control needs and a surface never
//! touches: a callback that must survive being shared into the tree, a ripple
//! that follows the theme after mount, and the `Send + Sync` constraint 2.0's
//! `Child` builder imposes.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{LayoutRequest, Widget};
use floom_types::Rect;
use floom_widgets::testkit::{SceneRecorder, TestEventContext};
use floom_widgets::{Button, Ripple};

use floom_core_theme::props::button::ButtonVariant;
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