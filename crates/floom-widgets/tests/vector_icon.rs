//! Scene and semantics snapshots for `VectorIcon`, the widget behind every
//! floem-ui icon.
//!
//! The thing worth pinning here is the *mapping*: an outline authored in a
//! 24x24 view box has to land on whatever box layout gave the widget, centred
//! and undistorted. A regression there is invisible in a type signature and
//! obvious on screen.

use floom_core::{Length, Rect, Widget};
use floom_types::{PathSeg, Point};
use floom_widgets::VectorIcon;
use floom_widgets::testkit::{
    AllocatingCreateContext, RecordingUpdateContext, SceneRecorder, paint_snapshot,
    semantics_snapshot,
};

const VIEW_BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 24.0,
    y1: 24.0,
};

/// A 24x24 triangle: move to the top-left corner, line to the bottom-right,
/// close. Two segments is enough to see where it landed.
const TRIANGLE: &str = "M0 0 L24 24 L0 24 Z";

fn allocate(widget: &dyn Widget) -> (floom_core::ElementId, AllocatingCreateContext) {
    let mut create = AllocatingCreateContext::new();
    let id = widget.create(&mut create);
    (id, create)
}

fn triangle() -> VectorIcon {
    VectorIcon::from_path_data(TRIANGLE, VIEW_BOX)
}

#[test]
fn an_icon_paints_a_filled_path() {
    let widget = triangle();
    let (id, _create) = allocate(&widget);
    let ops = paint_snapshot(&widget, id, VIEW_BOX);
    assert!(
        ops.starts_with("Path"),
        "expected a filled path, got {ops:?}"
    );
}

#[test]
fn the_outline_is_mapped_onto_the_laid_out_box() {
    // A 48x48 slot for a 24x24 view box is a clean 2x, so the corners land on
    // exact values and a wrong transform cannot hide behind rounding.
    let bounds = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 48.0,
        y1: 48.0,
    };
    let widget = triangle();
    let (id, _create) = allocate(&widget);

    let mut recorder = SceneRecorder::new(bounds);
    widget.paint(&mut recorder, id);
    recorder.finish();

    // Read the transform back off the widget rather than the scene: `fit` is
    // the mapping under test, and the scene node only proves it was consumed.
    let transform = widget.transform_for(bounds).expect("a square box fits");
    assert_eq!(
        transform.map_point(Point::new(0.0, 0.0)),
        Point::new(0.0, 0.0)
    );
    assert_eq!(
        transform.map_point(Point::new(24.0, 24.0)),
        Point::new(48.0, 48.0),
        "a 2x scale about the origin"
    );
}

#[test]
fn a_non_square_box_centres_the_icon_instead_of_stretching_it() {
    // 48x24 slot, 24x24 glyph: uniform scale means a 24x24 icon centred in the
    // 48-wide box. Stretching to fill would be the wrong answer, and the test
    // says so in the numbers.
    let bounds = Rect {
        x0: 10.0,
        y0: 0.0,
        x1: 58.0,
        y1: 24.0,
    };
    let widget = triangle();
    let transform = widget
        .transform_for(bounds)
        .expect("both axes are positive");
    let top_left = transform.map_point(Point::new(0.0, 0.0));
    let bottom_right = transform.map_point(Point::new(24.0, 24.0));

    assert!(
        (bottom_right.x - top_left.x - 24.0).abs() < 1e-9,
        "width must stay 24 (the glyph's own), got {}",
        bottom_right.x - top_left.x
    );
    assert!(
        (top_left.x - 22.0).abs() < 1e-9,
        "centred in the 48-wide slot means starting at x=22, got {}",
        top_left.x
    );
    assert!((top_left.y - bounds.y0).abs() < 1e-9, "fills the height");
}

#[test]
fn a_degenerate_box_yields_no_transform() {
    let widget = triangle();
    // A zero-height slot must not produce a NaN path.
    let flat = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 24.0,
        y1: 0.0,
    };
    assert!(widget.transform_for(flat).is_none());
    // Nor may a degenerate view box divide by zero.
    let empty_box = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 0.0,
        y1: 0.0,
    };
    assert!(widget.transform_for(empty_box).is_none());
}

#[test]
fn an_offset_view_box_is_honoured() {
    // `viewBox="-12 -12 24 24"` centres a glyph drawn around the origin; the
    // icon must land at the box's own top-left, not 12px off it.
    let view_box = Rect {
        x0: -12.0,
        y0: -12.0,
        x1: 12.0,
        y1: 12.0,
    };
    let widget = VectorIcon::from_path_data(TRIANGLE, view_box);
    let transform = widget.transform_for(VIEW_BOX).expect("square fits");
    let origin = transform.map_point(Point::new(-12.0, -12.0));
    assert!(
        (origin.x).abs() < 1e-9 && (origin.y).abs() < 1e-9,
        "the view box's own origin maps to the slot's, got {origin:?}"
    );
}

#[test]
fn intrinsic_size_is_the_configured_size_on_both_axes() {
    let widget = triangle().size(18.0);
    let req = widget
        .layout_request()
        .expect("an icon always wants a size");
    assert_eq!(req.width, Some(Length::Px(18.0)));
    assert_eq!(req.height, Some(Length::Px(18.0)));
}

#[test]
fn a_zero_size_wants_no_layout() {
    assert!(
        triangle().size(0.0).layout_request().is_none(),
        "a zero-sized icon has nothing to lay out"
    );
}

#[test]
fn unparseable_data_paints_nothing_instead_of_panicking() {
    let widget = VectorIcon::from_path_data("not a path", VIEW_BOX);
    let (id, _create) = allocate(&widget);
    assert!(widget.path().is_empty());
    assert_eq!(
        paint_snapshot(&widget, id, VIEW_BOX),
        "(empty)",
        "an empty path must emit no ops"
    );
}

#[test]
fn an_unlabelled_icon_is_decorative() {
    let widget = triangle();
    let (id, _create) = allocate(&widget);
    assert_eq!(semantics_snapshot(&widget, id, VIEW_BOX), "decorative");
}

#[test]
fn a_labelled_icon_reports_its_name_as_an_image() {
    let widget = triangle().label("Home");
    let (id, _create) = allocate(&widget);
    assert_eq!(
        semantics_snapshot(&widget, id, VIEW_BOX),
        r#"label="Home" (0.0, 0.0, 24.0, 24.0)"#
    );
}

#[test]
fn only_a_changed_icon_invalidates_its_element() {
    let mut create = AllocatingCreateContext::new();
    let mut widget = triangle();
    let id = widget.create(&mut create);

    let mut first = RecordingUpdateContext::new();
    widget.update(&mut first, id);

    let mut unchanged = RecordingUpdateContext::new();
    widget.update(&mut unchanged, id);
    assert_eq!(
        unchanged.dirty_count(),
        0,
        "an unchanged icon must not mark its element dirty"
    );

    let mut widget = widget.color(floom_core::Color::rgb(1, 2, 3));
    let mut changed = RecordingUpdateContext::new();
    widget.update(&mut changed, id);
    assert!(
        changed.dirty_count() > 0,
        "a colour change must repaint the glyph"
    );
}

#[test]
fn a_full_circle_icon_keeps_its_cubic_count() {
    // The Material `check_circle` glyph's outer ring is drawn with four arcs.
    // If arc support regressed to lines, this count would drop.
    let ring = VectorIcon::from_path_data(
        "M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2z",
        VIEW_BOX,
    );
    let cubics = ring
        .path()
        .segments()
        .iter()
        .filter(|s| matches!(s, PathSeg::CubicTo { .. }))
        .count();
    assert_eq!(cubics, 4, "four arcs, each one cubic");
}
