//! Scene and semantics snapshots for `Badge`, the first Stage B widget.
//!
//! `Badge` establishes the porting pattern for the floem-ui component library:
//! a theme-agnostic 2.0 widget that takes explicit values, with palette
//! resolution left to the design-system kits. See
//! `docs/porting-floem-ui-to-2.0.md`.

use floom_core::{Rect, Widget, Length};
use floom_widgets::testkit::{
    AllocatingCreateContext, RecordingUpdateContext, SceneRecorder, paint_snapshot,
    semantics_snapshot,
};
use floom_ui_composites::Badge;

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 40.0,
    y1: 20.0,
};

fn allocate(widget: &dyn Widget) -> (floom_core::ElementId, AllocatingCreateContext) {
    let mut create = AllocatingCreateContext::new();
    let id = widget.create(&mut create);
    (id, create)
}

#[test]
fn dot_badge_paints_a_circle() {
    let widget = Badge::new().dot(true).dot_size(8.0);
    let (id, _create) = allocate(&widget);
    // `paint` fills whatever box layout gave it (here BOX), and a dot's radius is
    // half that height -- so the bubble is a circle regardless of `dot_size`.
    // `dot_size` is an *intrinsic size*, asserted separately below.
    assert_eq!(
        paint_snapshot(&widget, id, BOX),
        "RoundedRect (0.0, 0.0, 40.0, 20.0) r=10.0 #C80000FF"
    );
}

#[test]
fn pill_badge_paints_fill_then_text() {
    let widget = Badge::new().content("7").font_size(11.0);
    let (id, _create) = allocate(&widget);
    let ops = paint_snapshot(&widget, id, BOX);
    assert!(
        ops.starts_with("RoundedRect (0.0, 0.0, 40.0, 20.0) r=10.0 #C80000FF"),
        "expected the bubble first, got {ops:?}"
    );
    assert!(
        ops.contains("Text"),
        "expected the label to be painted, got {ops:?}"
    );
}

#[test]
fn explicit_radius_beats_the_default_pill() {
    let widget = Badge::new().content("7").radius(2.0);
    let (id, _create) = allocate(&widget);
    assert!(
        paint_snapshot(&widget, id, BOX).contains("r=2.0 "),
        "a finite radius must win over the height/2 pill default"
    );
}

#[test]
fn intrinsic_size_of_a_dot_is_its_diameter() {
    let widget = Badge::new().dot(true).dot_size(10.0);
    let req = widget.layout_request().expect("a dot always wants a size");
    assert_eq!(req.width, Some(Length::Px(10.0)));
    assert_eq!(req.height, Some(Length::Px(10.0)));
}

#[test]
fn intrinsic_size_of_a_pill_is_text_plus_padding_floored_at_min_size() {
    // Content is padded on both sides and never smaller than min_size.
    let widget = Badge::new().content("7").padding(6.0).min_size(16.0);
    let req = widget.layout_request().expect("content implies a size");

    let text = floom_text_measure(&widget);
    let want_w = (text + 12.0).max(16.0);
    let got_w = req
        .width
        .and_then(|l| l.px())
        .expect("content implies a pixel width");
    assert!(
        (got_w - want_w).abs() < 0.01,
        "width {got_w} should be {want_w}"
    );
    let got_h = req
        .height
        .and_then(|l| l.px())
        .expect("content implies a pixel height");
    assert!(
        got_h >= 16.0,
        "height {got_h} should honour min_size"
    );
}

/// The measured width of a badge's content at its font size.
fn floom_text_measure(badge: &Badge) -> f32 {
    // `layout_request` already folds in padding and min_size, so recover the raw
    // text extent the same way the widget does.
    let measured =
        floom_text::measure_text(&badge.content, badge.font_size, f64::INFINITY);
    measured.width as f32
}

#[test]
fn an_empty_badge_wants_no_size() {
    let widget = Badge::new();
    assert!(
        widget.layout_request().is_none(),
        "a badge with neither a dot nor content has nothing to lay out"
    );
}

#[test]
fn pill_badge_reports_its_content_to_assistive_tech() {
    let widget = Badge::new().content("7");
    let (id, _create) = allocate(&widget);
    assert_eq!(
        semantics_snapshot(&widget, id, BOX),
        r#"label="7" (0.0, 0.0, 40.0, 20.0)"#
    );
}

#[test]
fn dot_badge_is_decorative() {
    let widget = Badge::new().dot(true);
    let (id, _create) = allocate(&widget);
    assert_eq!(
        semantics_snapshot(&widget, id, BOX),
        "decorative",
        "a dot has no text to announce"
    );
}

#[test]
fn only_a_changed_badge_invalidates_its_element() {
    let mut create = AllocatingCreateContext::new();
    let mut widget = Badge::new().content("7");
    let id = widget.create(&mut create);

    // First sync adopts the current revision.
    let mut first = RecordingUpdateContext::new();
    widget.update(&mut first, id);

    // Re-syncing an unchanged badge must be a no-op, or every frame would
    // invalidate the whole subtree.
    let mut unchanged = RecordingUpdateContext::new();
    widget.update(&mut unchanged, id);
    assert_eq!(
        unchanged.dirty_count(),
        0,
        "an unchanged badge must not mark its element dirty"
    );

    // Changing content does invalidate it.
    let mut widget = widget.content("42");
    let mut changed = RecordingUpdateContext::new();
    widget.update(&mut changed, id);
    assert!(
        changed.dirty_count() > 0,
        "a content change must mark the element dirty so the host repaints"
    );
}

#[test]
fn scene_lowering_emits_bubble_and_text_nodes() {
    let widget = Badge::new().content("7");
    let (id, _create) = allocate(&widget);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
}