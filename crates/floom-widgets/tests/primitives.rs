//! Scene and semantics snapshots for the Tier-0 primitives.

use floem_core::{Brush, Color, Point, Rect, ResourceId, Shadow, Size, Widget};
use floem_scene::SceneNode;
use floem_widgets::primitives::{Axis, Container, Divider, Icon, Image, ImageFit, Spacer, Text};
use floem_widgets::testkit::{
    AllocatingCreateContext, RecordingUpdateContext, SceneRecorder, paint_snapshot,
    semantics_snapshot,
};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 100.0,
    y1: 20.0,
};

const SQUARE: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 100.0,
    y1: 100.0,
};

fn allocate(widget: &dyn Widget) -> (floem_core::ElementId, AllocatingCreateContext) {
    let mut create = AllocatingCreateContext::new();
    let id = widget.create(&mut create);
    (id, create)
}

// --- Text -----------------------------------------------------------------

#[test]
fn text_scene_snapshot() {
    let widget = Text::new("Hello");
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, BOX),
        r#"Text (0.0, 0.0) 14.0 "Hello" #000000FF"#
    );
}

#[test]
fn text_semantics_snapshot() {
    let widget = Text::new("Hello");
    let (id, _create) = allocate(&widget);
    assert_eq!(
        semantics_snapshot(&widget, id, BOX),
        r#"label="Hello" (0.0, 0.0, 100.0, 20.0)"#
    );
}

#[test]
fn text_semantics_label_overrides_content() {
    let widget = Text::new("42").label("forty-two");
    let (id, _create) = allocate(&widget);
    assert_eq!(
        semantics_snapshot(&widget, id, BOX),
        r#"label="forty-two" (0.0, 0.0, 100.0, 20.0)"#
    );
}

#[test]
fn empty_text_emits_nothing() {
    let widget = Text::new("");
    let (id, _create) = allocate(&widget);
    assert_eq!(paint_snapshot(&widget, id, BOX), "(empty)");
    assert_eq!(semantics_snapshot(&widget, id, BOX), "decorative");
}

#[test]
fn text_content_change_marks_dirty_exactly_once() {
    let mut widget = Text::new("a");
    let (id, _create) = allocate(&widget);

    let mut update = RecordingUpdateContext::new();
    widget.update(&mut update, id);
    assert_eq!(update.dirty_count(), 1, "first update paints");

    widget.update(&mut update, id);
    assert_eq!(update.dirty_count(), 1, "steady state is clean");

    widget.set_content("ab");
    widget.update(&mut update, id);
    assert_eq!(update.dirty_count(), 2, "content change repaints once");

    widget.update(&mut update, id);
    assert_eq!(update.dirty_count(), 2);
}

// --- Icon -----------------------------------------------------------------

#[test]
fn icon_scene_snapshot() {
    let widget = Icon::new('\u{2605}').size(24.0);
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, BOX),
        "Text (0.0, 0.0) 24.0 \"\u{2605}\" #000000FF"
    );
}

#[test]
fn icon_is_decorative_without_label() {
    let widget = Icon::new('\u{2605}');
    let (id, _create) = allocate(&widget);
    assert_eq!(semantics_snapshot(&widget, id, BOX), "decorative");
}

#[test]
fn icon_label_promotes_it_to_accessible_text() {
    let widget = Icon::new('\u{2605}').label("Favorite");
    let (id, _create) = allocate(&widget);
    assert_eq!(
        semantics_snapshot(&widget, id, BOX),
        r#"label="Favorite" (0.0, 0.0, 100.0, 20.0)"#
    );
}

// --- Image ----------------------------------------------------------------

#[test]
fn image_contain_fit_snapshot() {
    let widget = Image::new(ResourceId::new(0, 1))
        .intrinsic_size(Size::new(200.0, 100.0))
        .fit(ImageFit::Contain);
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, SQUARE),
        "Image (0.0, 25.0, 100.0, 75.0) res=(0, 1)"
    );
}

#[test]
fn image_cover_fit_snapshot() {
    let widget = Image::new(ResourceId::new(0, 1))
        .intrinsic_size(Size::new(200.0, 100.0))
        .fit(ImageFit::Cover);
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, SQUARE),
        "Image (-50.0, 0.0, 150.0, 100.0) res=(0, 1)"
    );
}

#[test]
fn image_semantics_uses_alt_text() {
    let widget = Image::new(ResourceId::new(0, 1)).alt("A cat");
    let (id, _create) = allocate(&widget);
    assert_eq!(
        semantics_snapshot(&widget, id, SQUARE),
        r#"label="A cat" (0.0, 0.0, 100.0, 100.0)"#
    );
}

#[test]
fn image_without_alt_is_unnamed_but_still_present() {
    let widget = Image::new(ResourceId::new(0, 1));
    let (id, _create) = allocate(&widget);
    assert_eq!(
        semantics_snapshot(&widget, id, SQUARE),
        "unnamed (0.0, 0.0, 100.0, 100.0)"
    );
}

// --- Container ------------------------------------------------------------

#[test]
fn container_without_background_paints_nothing() {
    let widget = Container::new();
    let (id, _create) = allocate(&widget);
    assert_eq!(paint_snapshot(&widget, id, BOX), "(empty)");
    assert_eq!(semantics_snapshot(&widget, id, BOX), "decorative");
}

#[test]
fn container_rounded_background_snapshot() {
    let widget = Container::new()
        .background(Color::rgb(17, 34, 51))
        .radius(8.0);
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, BOX),
        "RoundedRect (0.0, 0.0, 100.0, 20.0) r=8.0 #112233FF"
    );
}

#[test]
fn container_zero_radius_uses_plain_rect() {
    let widget = Container::new().background(Color::WHITE);
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, BOX),
        "Rect (0.0, 0.0, 100.0, 20.0) #FFFFFFFF"
    );
}

/// A gradient background reaches the scene as a *brush*, not as one unlucky
/// colour the Layouter collapsed it to — the retained node carries the ramp,
/// because that is what every renderer needs to sample per pixel.
#[test]
fn container_gradient_background_reaches_the_scene_as_a_brush() {
    let ramp = Brush::linear(
        &[
            (0.0, Color::rgb(220, 40, 40)),
            (1.0, Color::rgb(40, 60, 220)),
        ],
        Point::new(0.0, 0.0),
        Point::new(100.0, 0.0),
    );
    let widget = Container::new().background(ramp.clone());
    let (id, _create) = allocate(&widget);

    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();

    let nodes: Vec<_> = recorder
        .scene()
        .iter()
        .map(|(_, node)| node.clone())
        .collect();
    assert_eq!(nodes.len(), 1, "one box, one node");
    match &nodes[0] {
        SceneNode::Rect { fill, shadow, .. } => {
            assert_eq!(fill, &ramp, "the whole ramp survives to the renderer");
            assert!(shadow.is_none(), "no shadow was asked for");
        }
        other => panic!("expected a rect node, got {other:?}"),
    }
    // The op log is what a one-line snapshot can carry: the colour at the
    // box's centre (50 of 100 along the axis ⇒ halfway between the two stops,
    // i.e. rgb(130, 50, 130)). It is an *approximation* of the ink, and it is
    // not where a test about the ramp itself should look — the node above is.
    assert_eq!(
        recorder.snapshot(),
        "BrushRect (0.0, 0.0, 100.0, 20.0) r=0.0 #823282FF"
    );
}

/// A shadow is cast *behind* the box's own fill: the op order inside one paint
/// call is the draw order the scene keeps, and a renderer that composites them
/// the other way round would hide the card under its own shadow.
#[test]
fn container_shadow_is_cast_behind_its_own_fill() {
    let shadow = Shadow {
        color: Color::rgba(0, 0, 0, 128),
        blur: 4.0,
        offset_x: 0.0,
        offset_y: 3.0,
    };
    let widget = Container::new()
        .background(Color::WHITE)
        .radius(6.0)
        .shadow(shadow);
    let (id, _create) = allocate(&widget);

    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();

    let lines: Vec<String> = recorder.snapshot().lines().map(|l| l.to_string()).collect();
    assert_eq!(
        lines,
        vec![
            "Shadow (0.0, 0.0, 100.0, 20.0) r=6.0 blur=4.0 off=(0.0, 3.0)".to_string(),
            "RoundedRect (0.0, 0.0, 100.0, 20.0) r=6.0 #FFFFFFFF".to_string(),
        ],
        "the ask precedes the shape it belongs to"
    );
    // And the two are one request each in the retained scene, not one merged
    // node a reader could mistake for the fill carrying the shadow silently.
    let carries: Vec<bool> = recorder
        .scene()
        .iter()
        .map(|(_, node)| {
            matches!(
                node,
                SceneNode::Rect {
                    shadow: Some(_),
                    ..
                }
            )
        })
        .collect();
    assert_eq!(carries, vec![true, false]);
    assert_eq!(widget.drop_shadow(), Some(shadow));
}

/// The fast path a solid fill takes is observable: exactly the ops it emitted
/// before gradients existed. Nothing about the new capability may quietly
/// reclassify yesterday's flat background.
#[test]
fn container_solid_background_is_still_a_plain_rect_op() {
    let widget = Container::new().background(Color::rgb(1, 2, 3));
    assert_eq!(widget.background_color(), Some(Color::rgb(1, 2, 3)));
    assert!(widget.background_brush().unwrap().solid().is_some());
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, BOX),
        "Rect (0.0, 0.0, 100.0, 20.0) #010203FF"
    );
}

// --- Spacer ---------------------------------------------------------------

#[test]
fn spacer_paints_and_describes_nothing() {
    let widget = Spacer::new(12.0);
    let (id, _create) = allocate(&widget);
    assert_eq!(paint_snapshot(&widget, id, BOX), "(empty)");
    assert_eq!(semantics_snapshot(&widget, id, BOX), "decorative");
    assert_eq!(widget.extent(), 12.0);
    assert!(!widget.is_flexible());
    assert!(Spacer::flexible().is_flexible());
}

// --- Divider --------------------------------------------------------------

#[test]
fn divider_horizontal_snapshot() {
    let widget = Divider::new();
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, BOX),
        "Rect (0.0, 9.5, 100.0, 10.5) #000000FF"
    );
}

#[test]
fn divider_vertical_snapshot_with_indent() {
    let widget = Divider::new().axis(Axis::Vertical).indent(4.0);
    let tall = Rect::from_origin_size(Point::ZERO, Size::new(20.0, 100.0));
    let (id, _create) = allocate(&widget);
    assert_eq!(
        paint_snapshot(&widget, id, tall),
        "Rect (9.5, 4.0, 10.5, 96.0) #000000FF"
    );
}

#[test]
fn divider_is_decorative() {
    let widget = Divider::new();
    let (id, _create) = allocate(&widget);
    assert_eq!(semantics_snapshot(&widget, id, BOX), "decorative");
}

// --- Scene lowering -------------------------------------------------------

#[test]
fn paint_ops_lower_into_retained_scene_nodes() {
    let cases: [(&dyn Widget, &str); 3] = [
        (&Text::new("Hi"), "Text"),
        (
            &Image::new(ResourceId::new(0, 1)).intrinsic_size(Size::new(10.0, 10.0)),
            "Image",
        ),
        (&Container::new().background(Color::BLACK), "Rect"),
    ];

    for (widget, expected_kind) in cases {
        let mut create = AllocatingCreateContext::new();
        let id = widget.create(&mut create);
        let mut recorder = SceneRecorder::new(SQUARE);
        widget.paint(&mut recorder, id);
        recorder.finish();

        assert_eq!(
            recorder.scene().len(),
            1,
            "{expected_kind} retains one node"
        );
        assert_eq!(recorder.scene_node_kinds(), vec![expected_kind]);
        assert_eq!(recorder.ops().len(), 1);
    }
}

#[test]
fn primitives_are_reusable_across_frames() {
    let mut widget = Container::new().background(Color::BLACK);
    let (id, _create) = allocate(&widget);
    let mut update = RecordingUpdateContext::new();

    let first = paint_snapshot(&widget, id, BOX);
    widget.update(&mut update, id);
    let second = paint_snapshot(&widget, id, BOX);
    assert_eq!(first, second, "painting is idempotent");
    assert_eq!(update.dirty_count(), 1);
}

// --- filters --------------------------------------------------------------

/// A backdrop blur is an *op*, not a declaration: its input is whatever is
/// already behind this box, which a bracket around the widget's own paint
/// cannot reach. So it has to appear in the op log, ahead of the fill that
/// lands on top of it.
#[test]
fn a_backdrop_blur_is_asked_for_before_the_fill_that_sits_on_top_of_it() {
    let widget = Container::new().background(Color::WHITE).backdrop_blur(6.0);
    let (id, _create) = allocate(&widget);

    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();

    let lines: Vec<String> = recorder.snapshot().lines().map(|l| l.to_string()).collect();
    assert_eq!(
        lines,
        vec![
            "BackdropFilter (0.0, 0.0, 100.0, 20.0) r=(6.0, 6.0)".to_string(),
            "Rect (0.0, 0.0, 100.0, 20.0) #FFFFFFFF".to_string(),
        ],
        "the backdrop is filtered before the card's own fill covers it"
    );
    assert_eq!(
        recorder.scene_node_kinds(),
        vec!["BackdropFilter", "Rect"],
        "and the retained scene carries it as its own node"
    );
}

/// A *content* blur is the other half, and it is deliberately **absent** from
/// the op log: the widget declares it and the host brackets the subtree, so a
/// paint call has nothing to emit. That absence is the assertion — a filter
/// pushed at paint time would blur only this widget's own nodes and leave its
/// children sharp.
#[test]
fn a_content_blur_is_declared_not_painted() {
    let widget = Container::new().background(Color::WHITE).blur(4.0);
    let (id, _create) = allocate(&widget);

    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();

    assert_eq!(
        recorder.scene_node_kinds(),
        vec!["Rect"],
        "the paint call emitted no filter marker"
    );
    assert_eq!(
        widget.filter(),
        Some(floem_core::Filter::blur(4.0)),
        "but the widget declares one, which is where the host reads it"
    );
    assert_eq!(widget.content_filter(), Some(floem_core::Filter::blur(4.0)));
}
