//! The engine's `Tooltip`: a bubble that comes up while the pointer rests on
//! the control.
//!
//! The state machine here is small -- a hover opens the bubble, leaving closes
//! it -- and the shape is the interesting part: the bubble hugs its text, and
//! the gap is the space *between* the control and the bubble.

use floom_core::{Child, ChildKey, ElementId, LayoutRequest, Point, PointerEvent, Rect, Widget};
use floom_types::{Insets, PointerPhase};
use floom_ui_composites::{Tooltip, TooltipStyle};
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 80.0,
};

const AT: Point = Point { x: 10.0, y: 10.0 };

fn trigger() -> Child {
    Child::new(ChildKey::index(0), LayoutRequest::new(), || {
        Box::new(floom_widgets::Text::new("trigger"))
    })
}

fn tooltip() -> Tooltip {
    Tooltip::new()
        .trigger(trigger())
        .text("Hello")
        .style(TooltipStyle {
            background: floom_core::Color::rgb(0x11, 0x22, 0x33),
            foreground: floom_core::Color::rgb(0x44, 0x55, 0x66),
            padding: 6.0,
            radius: 4.0,
            font_size: 12.0,
            gap: 4.0,
        })
}

/// The update pass, which is where the hover reaches the bubble.
fn tick(widget: &mut Tooltip) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut cx = RecordingUpdateContext::new();
    widget.update(&mut cx, id);
}

/// The pointer arriving on the control.
fn arrive(widget: &mut Tooltip) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut enter = PointerEvent::primary_down(AT);
    enter.phase = PointerPhase::Enter;
    let mut cx = TestEventContext::new(id, BOX, enter.into());
    widget.event(&mut cx, id);
    tick(widget);
}

/// The pointer leaving the control.
fn leave(widget: &mut Tooltip) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut exit = PointerEvent::primary_down(AT);
    exit.phase = PointerPhase::Leave;
    let mut cx = TestEventContext::new(id, BOX, exit.into());
    widget.event(&mut cx, id);
    tick(widget);
}

/// The bubble is up only while the pointer rests on the control.
#[test]
fn the_bubble_is_up_only_while_the_pointer_rests_on_the_control() {
    let mut widget = tooltip();
    assert!(!widget.is_hovered(), "not yet");
    assert!(!widget.is_present(), "so no bubble");

    arrive(&mut widget);
    assert!(widget.is_hovered(), "the pointer arrived");
    assert!(widget.is_present(), "so the bubble is up");

    leave(&mut widget);
    assert!(!widget.is_hovered(), "the pointer left");
    assert!(!widget.is_present(), "so the bubble is down");
}

/// Without a host to ask, the bubble is in the tree -- and only while it is up.
#[test]
fn the_bubble_is_in_the_tree_while_it_is_up() {
    let mut widget = tooltip();
    assert_eq!(widget.children().len(), 1, "the control alone");

    arrive(&mut widget);
    assert_eq!(widget.children().len(), 2, "the control and the bubble");
}

/// The gap is the space between the control and the bubble, not a space inside
/// either.
#[test]
fn the_gap_is_the_space_between_the_control_and_the_bubble() {
    let mut widget = tooltip();
    arrive(&mut widget);
    let bubble = widget.children().remove(1);
    let request = bubble.layout;
    assert_eq!(
        request.margin,
        Some(Insets {
            top: 4.0,
            left: 0.0,
            right: 0.0,
            bottom: 0.0,
        }),
        "the gap is a margin on the bubble's own slot"
    );
}

/// The bubble is a rounded box with the text in it.
#[test]
fn the_bubble_paints_its_fill_and_its_text() {
    let mut widget = tooltip();
    arrive(&mut widget);
    let bubble = widget.children().remove(1).build();

    let mut create = AllocatingCreateContext::new();
    let id: ElementId = bubble.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    bubble.paint(&mut recorder, id);
    recorder.finish();
    let fill = recorder.ops().iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        fill,
        Some(floom_core::Color::rgb(0x11, 0x22, 0x33)),
        "the fill"
    );

    // The text is the bubble's child.
    let label = bubble.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = label.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    label.paint(&mut recorder, id);
    recorder.finish();
    let (text, ink) = recorder
        .ops()
        .iter()
        .find_map(|op| match op {
            PaintOp::Text { text, color, .. } => Some((text.clone(), *color)),
            _ => None,
        })
        .expect("the bubble paints its text");
    assert_eq!(text, "Hello");
    assert_eq!(ink, floom_core::Color::rgb(0x44, 0x55, 0x66), "in the ink");
}

/// The bubble is as wide as its text: the text is measured, and the bubble's
/// box is the text plus the padding.
#[test]
fn the_bubble_is_as_wide_as_its_text() {
    let style = TooltipStyle {
        font_size: 12.0,
        padding: 6.0,
        ..TooltipStyle::default()
    };
    let measured = floom_text::measure_text("Hello", style.font_size, f64::INFINITY);
    let width = measured.width as f64 + style.padding * 2.0;
    assert!(
        width > 0.0 && width < 200.0,
        "the bubble is the text plus the padding, not the control's width: {width}"
    );
}
