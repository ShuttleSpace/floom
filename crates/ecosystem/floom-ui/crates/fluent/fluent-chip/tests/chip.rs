//! The ported Fluent chip -- a pill with a label in the middle, in one of

use floom_core::{ElementId, Point, PointerEvent, Rect, Widget};
use floom_types::Color;
use floom_ui_composites::Chip;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::MultiTheme;

use floom_widget_fluent_chip::{FluentChip, FluentChipStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 80.0,
};

const AT: Point = Point { x: 20.0, y: 20.0 };

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn chip() -> FluentChip {
    FluentChip::new().with_theme(theme())
}

fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn painted(chip: &Chip) -> (Option<Color>, Option<Color>, Option<(f64, Color)>) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = chip.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    chip.paint(&mut recorder, id);
    recorder.finish();

    let fill = recorder.ops().iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    let ink = recorder.ops().iter().find_map(|op| match op {
        PaintOp::Text { color, .. } => Some(*color),
        _ => None,
    });
    let outline = recorder.ops().iter().find_map(|op| match op {
        PaintOp::Path {
            stroke: Some((width, color)),
            ..
        } => Some((*width, *color)),
        _ => None,
    });
    (fill, ink, outline)
}

fn tap(chip: &mut Chip) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = chip.create(&mut create);
    for phase in [PointerEvent::primary_down(AT), PointerEvent::primary_up(AT)] {
        let mut cx = TestEventContext::new(id, BOX, phase.into());
        chip.event(&mut cx, id);
    }
}

#[test]
fn the_tokens_reach_the_engine_chip() {
    let style: FluentChipStyle = chip().resolved_style();
    let engine = chip().label("One").view();

    assert_eq!(engine.height, style.height, "the token's height");
    assert_eq!(
        engine.padding_horizontal, style.padding_horizontal,
        "the token's padding"
    );
    assert_eq!(engine.radius, style.border_radius, "the token's radius");
    assert_eq!(engine.font_size, style.font_size, "the token's font size");
    assert_eq!(engine.label, "One");
    assert_eq!(engine.hover, None, "the hover fill");
    assert_eq!(
        engine.font_weight,
        floom_types::FontWeight::Medium,
        "the label's weight"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(chip().resolved_style().design_system, "fluent");
}

#[test]
fn the_three_appearances_are_the_ones_v1_drew() {
    let plain = chip().resolved_style();
    let (fill, ink, _) = painted(&chip().label("One").view());
    assert_eq!(fill, Some(paint(plain.background)), "the unchosen fill");
    assert_eq!(ink, Some(paint(plain.text_color)), "and the unchosen ink");

    let chosen = chip().selected(true).resolved_style();
    let (fill, ink, _) = painted(&chip().selected(true).label("One").view());
    assert_eq!(fill, Some(paint(chosen.background)), "the chosen fill");
    assert_eq!(ink, Some(paint(chosen.text_color)), "and the chosen ink");

    let off = chip().disabled(true).resolved_style();
    let (fill, ink, _) = painted(&chip().disabled(true).label("One").view());
    assert_eq!(fill, Some(paint(off.background)), "the disabled fill");
    assert_eq!(ink, Some(paint(off.text_color)), "and the disabled ink");
}

#[test]
fn the_outline_is_the_one_v1_drew() {
    let (fill, ink, outline) = painted(&chip().label("One").view());
    assert!(fill.is_some(), "the pill is filled");
    assert!(ink.is_some(), "and the label is drawn");
    assert!(
        outline.is_none(),
        "v1's Fluent chip drew an outline only when it was chosen"
    );
    let chosen = chip().selected(true).resolved_style();
    let (_, _, outline) = painted(&chip().selected(true).label("One").view());
    let (width, color) = outline.expect("an outline on the chosen chip");
    assert_eq!(width, 1.0, "v1 wrote `.border(1.0)`");
    assert_eq!(color, paint(chosen.border_color), "in `primary`");
}

#[test]
fn the_label_is_the_chips_own_text() {
    let mut create = AllocatingCreateContext::new();
    let widget = chip().label("Hello").view();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    let text = recorder
        .ops()
        .iter()
        .find_map(|op| match op {
            PaintOp::Text { text, size, .. } => Some((text.clone(), *size)),
            _ => None,
        })
        .expect("a chip paints its label");
    assert_eq!(text.0, "Hello");
    assert_eq!(text.1, chip().resolved_style().font_size);
}

#[test]
fn a_click_calls_the_callback() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(0));
    let s = seen.clone();
    let mut widget = chip()
        .label("One")
        .on_click(move || *s.lock().unwrap() += 1)
        .view();
    tap(&mut widget);
    assert_eq!(*seen.lock().unwrap(), 1);
}

#[test]
fn a_disabled_chip_does_not_call_the_callback() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(0));
    let s = seen.clone();
    let mut widget = chip()
        .label("One")
        .disabled(true)
        .on_click(move || *s.lock().unwrap() += 1)
        .view();
    tap(&mut widget);
    assert_eq!(*seen.lock().unwrap(), 0, "v1's `if !disabled` guard");
}

#[test]
fn the_props_report_the_shape_and_not_the_callback() {
    let kit = chip().label("One").selected(true).disabled(true);
    assert_eq!(kit.props().label, "One");
    assert!(kit.props().selected);
    assert!(kit.props().disabled);
    assert!(kit.props().on_click.is_none());
}
