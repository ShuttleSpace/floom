//! The ported Fluent checkbox button -- a chip-shaped toggle: a rounded box

use floom_core::{ElementId, Point, PointerEvent, Rect, Widget, WidgetAny};
use floom_types::Color;
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};
use floom_widgets::Stack;

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_checkbox_btn::{FluentCheckboxBtn, FluentCheckboxBtnStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 80.0,
};

const AT: Point = Point { x: 20.0, y: 20.0 };

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentCheckboxBtn {
    FluentCheckboxBtn::new().with_theme(theme())
}

fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn ops_of(widget: &dyn Widget) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

fn mounted_pill(widget: &Stack) -> Box<dyn WidgetAny> {
    for child in widget.children() {
        let mut conditional = child.build();
        let mut create = AllocatingCreateContext::new();
        let id = conditional.create(&mut create);
        let mut cx = RecordingUpdateContext::new();
        conditional.update(&mut cx, id);
        if let Some(child) = conditional.children().first() {
            return child.build();
        }
    }
    panic!("no pill is mounted");
}

fn pill_surface(widget: &Stack) -> Box<dyn WidgetAny> {
    let pressable = mounted_pill(widget);
    pressable.children().remove(0).build()
}

fn pill_content(widget: &Stack) -> Box<dyn WidgetAny> {
    let surface = pill_surface(widget);
    surface.children().remove(0).build()
}

fn pill_mark(widget: &Stack) -> Box<dyn WidgetAny> {
    let content = pill_content(widget);
    content.children().remove(0).build()
}

fn pill_label(widget: &Stack) -> Box<dyn WidgetAny> {
    let content = pill_content(widget);
    content.children().remove(1).build()
}

fn pill_painted(widget: &Stack) -> (Option<Color>, Option<(f64, Color)>) {
    let surface = pill_surface(widget);
    let ops = ops_of(&*surface);
    let fill = ops.iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    let rim = ops.iter().find_map(|op| match op {
        PaintOp::Path {
            stroke: Some((width, color)),
            ..
        } => Some((*width, *color)),
        _ => None,
    });
    (fill, rim)
}

fn mark_ink(widget: &Stack) -> Option<Color> {
    let mark = pill_mark(widget);
    let ops = ops_of(&*mark);
    ops.iter().find_map(|op| match op {
        PaintOp::Path { fill, .. } => *fill,
        _ => None,
    })
}

fn label_painted(widget: &Stack) -> (String, f32, Color) {
    let label = pill_label(widget);
    let ops = ops_of(&*label);
    ops.iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("a checkbox button paints its label")
}

fn tap(widget: &Stack) {
    let mut pressable = mounted_pill(widget);
    let mut create = AllocatingCreateContext::new();
    let id = pressable.create(&mut create);
    for phase in [PointerEvent::primary_down(AT), PointerEvent::primary_up(AT)] {
        let mut cx = TestEventContext::new(id, BOX, phase.into());
        pressable.event(&mut cx, id);
    }
}


#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: FluentCheckboxBtnStyle = kit().resolved_style();
    let tokens =
        match theme().component(floom_core_theme::component_tokens::ComponentId::CheckboxBtn) {
            floom_core_theme::component_tokens::ComponentTokens::CheckboxBtn(tokens) => tokens,
            _ => unreachable!(),
        };
    assert_eq!(style.height, tokens.height, "the token's height");
    assert_eq!(style.padding, tokens.padding, "the token's padding");
    assert_eq!(style.radius, tokens.radius, "the token's radius");
    assert_eq!(style.gap, tokens.gap, "the token's gap");
    assert_eq!(style.font_size, tokens.font_size, "the token's font size");
    assert_eq!(
        style.background,
        theme().colors().surface_variant,
        "the pill's fill"
    );
    assert_eq!(
        style.border_color,
        theme().colors().outline,
        "the pill's rim"
    );
    assert_eq!(
        style.checked_background,
        theme().colors().primary,
        "the checked fill"
    );
    assert_eq!(
        style.checked_border_color,
        theme().colors().primary,
        "the checked rim"
    );
    assert_eq!(
        style.mark_color,
        theme().colors().on_primary,
        "the mark's ink"
    );
}

#[test]
fn the_numbers_v1_wrote_as_literals_are_literals_here() {
    let style = kit().resolved_style();
    assert_eq!(style.border_width, 1.0, "v1's `.border(1.0)`");
    assert_eq!(
        style.mark_size,
        style.font_size * 1.2,
        "v1's `font_size * 1.2`"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}


#[test]
fn the_unchecked_pill_paints_v1_s_fill_and_rim() {
    let style = kit().resolved_style();
    let widget = kit().label("Notify").view();
    let (fill, rim) = pill_painted(&widget);
    assert_eq!(fill, Some(paint(style.background)), "the pill's fill");
    assert_eq!(
        rim,
        Some((f64::from(style.border_width), paint(style.border_color))),
        "the pill's rim"
    );
}

#[test]
fn the_checked_pill_paints_v1_s_fill_and_rim() {
    let style = kit().resolved_style();
    let widget = kit().checked(true).label("Notify").view();
    let (fill, rim) = pill_painted(&widget);
    assert_eq!(
        fill,
        Some(paint(style.checked_background)),
        "the pill's fill"
    );
    assert_eq!(
        rim,
        Some((
            f64::from(style.border_width),
            paint(style.checked_border_color)
        )),
        "the pill's rim"
    );
}

#[test]
fn the_disabled_pill_paints_v1_s_disabled_fill_and_rim() {
    let style = kit().resolved_style();
    let widget = kit().checked(true).disabled(true).label("Notify").view();
    let (fill, rim) = pill_painted(&widget);
    assert_eq!(
        fill,
        Some(paint(style.disabled_background)),
        "the pill's fill"
    );
    assert_eq!(
        rim,
        Some((
            f64::from(style.border_width),
            paint(style.disabled_border_color)
        )),
        "the pill's rim"
    );
    assert_eq!(style.disabled_background.a, 12, "v1's disabled fill");
    assert_eq!(style.disabled_border_color.a, 38, "v1's disabled rim");
}

#[test]
fn the_mark_is_transparent_while_unchecked() {
    let style = kit().resolved_style();
    let widget = kit().label("Notify").view();
    assert_eq!(
        mark_ink(&widget),
        Some(paint(style.unchecked_mark_color)),
        "v1 drew the unchecked mark transparent"
    );
}

#[test]
fn the_mark_is_on_primary_while_checked() {
    let style = kit().resolved_style();
    let widget = kit().checked(true).label("Notify").view();
    assert_eq!(mark_ink(&widget), Some(paint(style.mark_color)));
}

#[test]
fn the_mark_is_the_disabled_tint_while_unchecked_and_disabled() {
    let style = kit().resolved_style();
    let widget = kit().disabled(true).label("Notify").view();
    assert_eq!(
        mark_ink(&widget),
        Some(paint(style.unchecked_disabled_mark_color)),
        "Fluent wrote the disabled tint outside the checked branch"
    );
    assert_eq!(
        style.unchecked_disabled_mark_color.a, 38,
        "so the unchecked and disabled mark is a grey check here"
    );
}

#[test]
fn the_label_is_painted_with_v1_s_text_size_and_ink() {
    let style = kit().resolved_style();
    let widget = kit().label("Notify").view();
    let (text, size, ink) = label_painted(&widget);
    assert_eq!(text, "Notify");
    assert_eq!(size, style.font_size, "the token's font size");
    assert_eq!(ink, paint(style.label_color), "v1's `on_surface`");
}

#[test]
fn the_disabled_label_is_painted_with_v1_s_disabled_ink() {
    let style = kit().resolved_style();
    let widget = kit().disabled(true).label("Notify").view();
    let (_, _, ink) = label_painted(&widget);
    assert_eq!(
        ink,
        paint(style.disabled_label_color),
        "v1's `on_surface` at 38"
    );
}


#[test]
fn clicking_the_pill_switches_it_on() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = seen.clone();
    let kit = kit()
        .label("Notify")
        .on_change(move |checked| s.lock().unwrap().push(checked));
    let watcher = kit.clone();
    let widget = kit.view();

    assert!(!watcher.is_checked(), "it starts unchecked");
    tap(&widget);
    assert!(watcher.is_checked(), "and the click switched it on");
    assert_eq!(seen.lock().unwrap().as_slice(), &[true], "v1's `cb(next)`");
}

#[test]
fn clicking_the_pill_again_switches_it_off() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = seen.clone();
    let kit = kit()
        .checked(true)
        .label("Notify")
        .on_change(move |checked| s.lock().unwrap().push(checked));
    let watcher = kit.clone();
    let widget = kit.view();

    tap(&widget);
    assert!(!watcher.is_checked());
    assert_eq!(seen.lock().unwrap().as_slice(), &[false]);
}

#[test]
fn the_pill_s_appearance_follows_the_state() {
    let style = kit().resolved_style();
    let kit = kit().label("Notify");
    let watcher = kit.clone();
    let widget = kit.view();

    let (fill, _) = pill_painted(&widget);
    assert_eq!(
        fill,
        Some(paint(style.background)),
        "unchecked to begin with"
    );

    tap(&widget);
    let (fill, _) = pill_painted(&widget);
    assert_eq!(
        fill,
        Some(paint(style.checked_background)),
        "and checked once the state has moved"
    );
}

#[test]
fn a_disabled_pill_does_not_answer() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = seen.clone();
    let kit = kit()
        .disabled(true)
        .label("Notify")
        .on_change(move |checked| s.lock().unwrap().push(checked));
    let watcher = kit.clone();
    let widget = kit.view();

    tap(&widget);
    assert!(
        !watcher.is_checked(),
        "v1's toggle was guarded by `if !disabled`"
    );
    assert!(seen.lock().unwrap().is_empty());
}


#[test]
fn the_props_report_the_shape_and_not_the_callback() {
    let kit = kit().checked(true).disabled(true).label("Notify");
    assert!(kit.props().checked);
    assert!(kit.props().disabled);
    assert_eq!(kit.props().label.as_deref(), Some("Notify"));
    assert!(kit.props().on_change.is_none());
}


#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    assert_ne!(
        FluentCheckboxBtn::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent checkbox button took a snapshot, not the app-wide signal"
    );
}
