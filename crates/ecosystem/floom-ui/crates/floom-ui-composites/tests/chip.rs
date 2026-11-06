use std::sync::Arc;
use std::sync::Mutex;

use floom_core::{Length, Point, PointerEvent, Rect, Widget};
use floom_types::PointerPhase;
use floom_ui_composites::Chip;
use floom_widgets::testkit::{
    paint_snapshot, AllocatingCreateContext, SceneRecorder, TestEventContext,
};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 120.0,
    y1: 32.0,
};

const AT: Point = Point { x: 10.0, y: 10.0 };

fn allocate(widget: &dyn Widget) -> (floom_core::ElementId, AllocatingCreateContext) {
    let mut create = AllocatingCreateContext::new();
    let id = widget.create(&mut create);
    (id, create)
}

/// The width a label wants at a size, from the engine's own measure.
fn text_width(label: &str, size: f32) -> f64 {
    floom_text::measure_text(label, size, f64::INFINITY).width as f64
}

/// The chip: the label plus the padding at each side, and the chip's height.
#[test]
fn the_intrinsic_size_is_the_label_plus_the_padding() {
    let widget = Chip::new()
        .label("Hi")
        .font_size(14.0)
        .padding_horizontal(12.0);
    let request = widget.layout_request().expect("a chip wants a size");
    assert_eq!(
        request.width,
        Some(Length::Px(text_width("Hi", 14.0) as f32 + 24.0)),
        "the label plus the padding at each side"
    );
    assert_eq!(
        request.height,
        Some(Length::Px(32.0)),
        "and the chip's height"
    );
}

/// The pill is filled, and the label is drawn in the middle of it.
#[test]
fn the_pill_is_filled_and_the_label_is_drawn() {
    let widget = Chip::new()
        .label("Hi")
        .background(floom_core::Color::rgb(0x11, 0x22, 0x33))
        .foreground(floom_core::Color::rgb(0x44, 0x55, 0x66))
        .radius(16.0);
    let (id, _create) = allocate(&widget);
    let ops = paint_snapshot(&widget, id, BOX);
    assert!(
        ops.contains("RoundedRect"),
        "the pill is filled, got {ops:?}"
    );
    assert!(ops.contains("Text"), "and the label is drawn, got {ops:?}");
    assert!(
        ops.contains("#445566FF"),
        "in the foreground colour, got {ops:?}"
    );
}

/// The label is centred in the pill, not at its edge.
#[test]
fn the_label_is_centred_in_the_pill() {
    let widget = Chip::new().label("Hi").font_size(14.0);
    let (id, _create) = allocate(&widget);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    let origin = recorder
        .ops()
        .iter()
        .find_map(|op| match op {
            floom_widgets::testkit::PaintOp::Text { origin, .. } => Some(*origin),
            _ => None,
        })
        .expect("a label is painted");
    let width = text_width("Hi", 14.0);
    let expected = (BOX.width() - width) / 2.0;
    assert!(
        (origin.x - BOX.x0 - expected).abs() < 0.5,
        "the label starts at {expected} from the edge, not at {}",
        origin.x - BOX.x0
    );
}

/// The outline is a stroke just inside the pill's edge.
#[test]
fn the_outline_is_stroked_inside_the_edge() {
    let widget = Chip::new()
        .label("Hi")
        .radius(16.0)
        .border(1.0, floom_core::Color::rgb(0xFF, 0x00, 0x00));
    let (id, _create) = allocate(&widget);
    let ops = paint_snapshot(&widget, id, BOX);
    assert!(ops.contains("Path"), "the outline is a path, got {ops:?}");
    assert!(
        ops.contains("#FF0000FF"),
        "in the border colour, got {ops:?}"
    );
}

/// A chip with no outline draws none.
#[test]
fn a_chip_with_no_outline_draws_none() {
    let widget = Chip::new().label("Hi");
    let (id, _create) = allocate(&widget);
    let ops = paint_snapshot(&widget, id, BOX);
    assert!(!ops.contains("Path"), "no outline, got {ops:?}");
}

/// The hover fill replaces the resting one while the pointer is over the chip.
#[test]
fn the_hover_fill_replaces_the_resting_one() {
    let mut widget = Chip::new()
        .label("Hi")
        .background(floom_core::Color::rgb(0x11, 0x22, 0x33))
        .hover(Some(floom_core::Color::rgb(0x77, 0x88, 0x99)));
    let (id, _create) = allocate(&widget);
    let resting = paint_snapshot(&widget, id, BOX);
    assert!(
        resting.contains("#112233FF"),
        "the resting fill, got {resting:?}"
    );

    let mut enter = PointerEvent::primary_down(AT);
    enter.phase = PointerPhase::Enter;
    let mut cx = TestEventContext::new(id, BOX, enter.into());
    widget.event(&mut cx, id);

    let hovered = paint_snapshot(&widget, id, BOX);
    assert!(
        hovered.contains("#778899FF"),
        "the hover fill, got {hovered:?}"
    );
}

/// A click calls the callback.
#[test]
fn a_click_calls_the_callback() {
    let seen = Arc::new(Mutex::new(0));
    let s = seen.clone();
    let mut widget = Chip::new()
        .label("Hi")
        .on_click(move || *s.lock().unwrap() += 1);
    let (id, _create) = allocate(&widget);
    for phase in [PointerEvent::primary_down(AT), PointerEvent::primary_up(AT)] {
        let mut cx = TestEventContext::new(id, BOX, phase.into());
        widget.event(&mut cx, id);
    }
    assert_eq!(*seen.lock().unwrap(), 1);
}

/// A disabled chip does not.
#[test]
fn a_disabled_chip_does_not_call_the_callback() {
    let seen = Arc::new(Mutex::new(0));
    let s = seen.clone();
    let mut widget = Chip::new()
        .label("Hi")
        .disabled(true)
        .on_click(move || *s.lock().unwrap() += 1);
    let (id, _create) = allocate(&widget);
    for phase in [PointerEvent::primary_down(AT), PointerEvent::primary_up(AT)] {
        let mut cx = TestEventContext::new(id, BOX, phase.into());
        widget.event(&mut cx, id);
    }
    assert_eq!(*seen.lock().unwrap(), 0, "a disabled chip does not answer");
}

/// The semantics: a label, and a button when it can be clicked.
#[test]
fn the_semantics_report_the_label() {
    let widget = Chip::new().label("Hi").on_click(|| {});
    let hint = widget.semantics_hint();
    assert_eq!(hint.label.as_deref(), Some("Hi"));
    assert_eq!(
        hint.role,
        floom_accessibility::SemanticsRole::Button,
        "a clickable chip is a button"
    );

    let plain = Chip::new().label("Hi");
    assert_eq!(
        plain.semantics_hint().role,
        floom_accessibility::SemanticsRole::StaticText,
        "a chip that cannot be clicked is a label"
    );

    let empty = Chip::new();
    assert!(
        empty.semantics_hint().label.is_none(),
        "a chip with no label has nothing to announce"
    );
}
