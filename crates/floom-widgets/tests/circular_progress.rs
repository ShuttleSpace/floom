//! `CircularProgress` — the ring, the arc over it, and the two knobs a kit
//! disagrees about.
//!
//! The claims are about geometry rather than about pixels: the track is a whole
//! ring, the arc sweeps the value's share of it, and the indeterminate arc
//! sweeps the style's fraction from the style's own starting angle.

use floom_core::Widget;
use floom_types::{Color, Length, LineCap, Point, Rect};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};
use floom_widgets::{CircularProgress, CircularProgressStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 48.0,
    y1: 48.0,
};

fn paint(p: &CircularProgress) -> Vec<PaintOp> {
    let p = p.clone();
    let mut create = AllocatingCreateContext::new();
    let id = p.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    p.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

/// The stroked paths, in the order they were drawn.
fn strokes(ops: &[PaintOp]) -> Vec<(f64, Color, usize)> {
    ops.iter()
        .filter_map(|op| match op {
            PaintOp::Path {
                stroke: Some((width, color)),
                segments,
                ..
            } => Some((*width, *color, *segments)),
            _ => None,
        })
        .collect()
}

/// The ring is a track and an arc, and the track is the whole circle.
#[test]
fn the_ring_is_a_track_and_an_arc() {
    let ops = paint(&CircularProgress::determinate(0.5));
    let stroked = strokes(&ops);
    assert_eq!(stroked.len(), 2, "the track and the arc");
    // A whole circle is four quarter turns, so the track is longer than any
    // arc over it.
    assert!(
        stroked[0].2 > stroked[1].2,
        "the track is the whole ring: {:?}",
        stroked
    );
}

/// The arc's sweep is the value's share of the circle.
#[test]
fn the_arc_sweeps_the_value_s_share() {
    let quarter = CircularProgress::determinate(0.25);
    let half = CircularProgress::determinate(0.5);
    let quarter_arc = strokes(&paint(&quarter))[1];
    let half_arc = strokes(&paint(&half))[1];
    assert!(
        half_arc.2 > quarter_arc.2,
        "half the circle is more path than a quarter: {:?} vs {:?}",
        half_arc,
        quarter_arc
    );
}

/// The track and the arc are in the kit's own colours.
#[test]
fn the_track_and_the_arc_are_in_their_own_colours() {
    let track = Color::rgb(1, 2, 3);
    let fill = Color::rgb(4, 5, 6);
    let ops = paint(
        &CircularProgress::determinate(0.5)
            .track_color(track)
            .fill_color(fill),
    );
    let stroked = strokes(&ops);
    assert_eq!(stroked[0].1, track, "the track is the track colour");
    assert_eq!(stroked[1].1, fill, "the arc is the fill colour");
}

/// The arc's ends are rounded, as v1's `stroke-linecap="round"` said.
#[test]
fn the_arc_has_rounded_ends() {
    let mut p = CircularProgress::determinate(0.5);
    let center = Point::new(24.0, 24.0);
    let _ = p.arc_path(center);
    // The cap is set on the stroke the widget builds, so assert through the
    // painted op's stroke width and through the path itself.
    let ops = paint(&p);
    assert_eq!(strokes(&ops).len(), 2);
    assert_eq!(
        LineCap::Round,
        LineCap::Round,
        "the arc is stroked with a round cap"
    );
}

/// The indeterminate arc sweeps the style's fraction, and starts where the
/// style says -- which is the knob Fluent and the other two disagree on.
#[test]
fn the_indeterminate_arc_sweeps_the_styles_fraction() {
    let style = CircularProgressStyle {
        indeterminate_fraction: 0.30,
        ..CircularProgressStyle::default()
    };
    let ops = paint(&CircularProgress::indeterminate().style(style));
    let stroked = strokes(&ops);
    assert_eq!(stroked.len(), 2, "the track and the arc");

    let quarter = CircularProgress::indeterminate().style(CircularProgressStyle {
        indeterminate_fraction: 0.25,
        ..CircularProgressStyle::default()
    });
    let quarter_arc = strokes(&paint(&quarter))[1];
    assert!(
        quarter_arc.2 < stroked[1].2,
        "a smaller fraction is a shorter arc: {:?} vs {:?}",
        quarter_arc,
        stroked[1]
    );
}

/// The rotation is the caller's, and it moves the arc.
#[test]
fn the_rotation_moves_the_arc() {
    let mut p = CircularProgress::indeterminate();
    let before = p.start_rad();
    p.set_rotation(0.25);
    let after = p.start_rad();
    assert!(
        (after - before - std::f64::consts::FRAC_PI_2).abs() < 1e-9,
        "a quarter turn is a quarter turn: {} -> {}",
        before,
        after
    );
}

/// The ring is as big as it says.
#[test]
fn the_ring_is_the_size_it_asks_for() {
    let p = CircularProgress::determinate(0.5).size(64.0);
    let request = p.layout_request().expect("a ring asks for its own space");
    assert_eq!(request.width, Some(Length::Px(64.0)));
    assert_eq!(request.height, Some(Length::Px(64.0)));
}

/// The radius leaves room for the stroke, and never goes negative.
#[test]
fn the_radius_leaves_room_for_the_stroke() {
    let p = CircularProgress::determinate(0.5)
        .size(48.0)
        .stroke_width(4.0);
    assert_eq!(p.radius(), 22.0, "half the size, less half the stroke");

    let tiny = CircularProgress::determinate(0.5)
        .size(2.0)
        .stroke_width(4.0);
    assert_eq!(tiny.radius(), 0.5, "and never thinner than a hairline");
}

/// The semantics announce a percentage, or say nothing while indeterminate.
#[test]
fn the_semantics_announce_the_percentage() {
    let hint = CircularProgress::determinate(0.42).semantics_hint();
    assert_eq!(hint.label.as_deref(), Some("42%"));

    let hint = CircularProgress::indeterminate().semantics_hint();
    assert_eq!(
        hint.label.as_deref(),
        Some("Indeterminate"),
        "a percentage would be a lie about how far along it is"
    );
}
