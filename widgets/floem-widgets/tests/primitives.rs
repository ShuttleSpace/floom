//! Scene and semantics snapshots for the Tier-0 primitives.

use floem_core::{Color, Point, Rect, ResourceId, Size, Widget};
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
