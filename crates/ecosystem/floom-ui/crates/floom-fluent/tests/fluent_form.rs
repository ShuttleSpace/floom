#![cfg(feature = "form")]
//! The ported Fluent form -- the caller's own fields, an error summary, and a

use floom_core::{
    Child, ChildKey, ElementId, LayoutRequest, Point, PointerEvent, Rect, Widget, WidgetAny,
};
use floom_types::Color;
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_fluent::widgets::form::{FluentForm, FluentFormStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 200.0,
};

const AT: Point = Point { x: 20.0, y: 20.0 };

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentForm {
    FluentForm::new().with_theme(theme())
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

fn form_content(kit: FluentForm) -> Box<dyn WidgetAny> {
    let surface = kit.view();
    surface.children().remove(0).build()
}

fn error_summary(kit: FluentForm) -> Box<dyn WidgetAny> {
    let content = form_content(kit);
    content.children().remove(1).build()
}

fn submit_slot(kit: FluentForm) -> Box<dyn WidgetAny> {
    let content = form_content(kit);
    content.children().remove(2).build()
}

fn mounted_submit(slot: &Box<dyn WidgetAny>) -> Box<dyn WidgetAny> {
    for child in slot.children() {
        let mut conditional = child.build();
        let mut create = AllocatingCreateContext::new();
        let id = conditional.create(&mut create);
        let mut cx = RecordingUpdateContext::new();
        conditional.update(&mut cx, id);
        if let Some(child) = conditional.children().first() {
            return child.build();
        }
    }
    panic!("no submit button is mounted");
}

fn submit_surface(slot: &Box<dyn WidgetAny>) -> Box<dyn WidgetAny> {
    let pressable = mounted_submit(slot);
    pressable.children().remove(0).build()
}

fn submit_fill(slot: &Box<dyn WidgetAny>) -> Option<Color> {
    let surface = submit_surface(slot);
    let ops = ops_of(&*surface);
    ops.iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    })
}

fn submit_label_painted(slot: &Box<dyn WidgetAny>) -> (String, f32, Color) {
    let surface = submit_surface(slot);
    let content = surface.children().remove(0).build();
    let text = content.children().remove(0).build();
    let ops = ops_of(&*text);
    ops.iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("the submit button paints its label")
}

fn error_messages_painted(kit: FluentForm) -> Vec<(String, f32, Color)> {
    let summary = error_summary(kit);
    let mut out = Vec::new();
    for child in summary.children() {
        let text = child.build();
        let ops = ops_of(&*text);
        if let Some((text, size, color)) = ops.iter().find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        }) {
            out.push((text, size, color));
        }
    }
    out
}

fn tap_submit(slot: &Box<dyn WidgetAny>) {
    let mut pressable = mounted_submit(slot);
    let mut create = AllocatingCreateContext::new();
    let id = pressable.create(&mut create);
    for phase in [PointerEvent::primary_down(AT), PointerEvent::primary_up(AT)] {
        let mut cx = TestEventContext::new(id, BOX, phase.into());
        pressable.event(&mut cx, id);
    }
}

#[test]
fn the_tokens_and_the_props_reach_the_style() {
    let style: FluentFormStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Form) {
        floom_core_theme::component_tokens::ComponentTokens::Form(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(
        style.submit_height, tokens.submit_height,
        "the token's height"
    );
    assert_eq!(
        style.submit_radius, tokens.submit_radius,
        "the token's radius"
    );
    assert_eq!(
        style.surface,
        theme().colors().surface,
        "the surface's fill"
    );
    assert_eq!(
        style.radius,
        theme().radius().md,
        "the theme's radius scale"
    );
}

#[test]
fn the_padding_and_the_gap_come_from_the_props_as_v1_read_them() {
    let kit = kit();
    let style = kit.resolved_style();
    assert_eq!(style.pad, kit.props().pad, "v1's `props.pad`");
    assert_eq!(style.gap, kit.props().gap, "v1's `props.gap`");
    assert_eq!(style.pad, 16.0, "the props' default");
    assert_eq!(style.gap, 12.0, "the props' default");
}

#[test]
fn the_validation_messages_token_reaches_the_style() {
    let style = kit().resolved_style();
    let vmt = match theme()
        .component(floom_core_theme::component_tokens::ComponentId::ValidationMessages)
    {
        floom_core_theme::component_tokens::ComponentTokens::ValidationMessages(vmt) => vmt,
        _ => unreachable!(),
    };
    assert_eq!(style.error_font_size, vmt.font_size, "the token's size");
    assert_eq!(style.error_gap, vmt.gap, "the token's gap");
    assert_eq!(style.error_color, theme().colors().error, "the error ink");
}

#[test]
fn the_numbers_v1_wrote_as_literals_are_literals_here() {
    let style = kit().resolved_style();
    assert_eq!(style.submit_font_size, 14.0, "v1's `.font_size(14.0)`");
    assert_eq!(
        style.submit_padding_horizontal, 24.0,
        "v1's `.padding_horiz(24.0)`"
    );
    assert_eq!(
        style.submit_font_weight,
        floom_types::FontWeight::Medium,
        "v1's medium"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}

#[test]
fn the_form_surface_is_painted_in_v1_s_fill() {
    let style = kit().resolved_style();
    let surface = kit().view();
    let ops = ops_of(&surface);
    let fill = ops.iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(fill, Some(paint(style.surface)), "the form's fill");
}

#[test]
fn the_valid_submit_button_paints_primary_and_the_invalid_one_does_not() {
    let style = kit().resolved_style();

    let valid = kit();
    let slot = submit_slot(valid);
    assert_eq!(
        submit_fill(&slot),
        Some(paint(style.submit_background)),
        "the valid button's fill"
    );

    let invalid = kit();
    invalid.set_valid(false);
    let slot = submit_slot(invalid);
    assert_eq!(
        submit_fill(&slot),
        Some(paint(style.invalid_submit_background)),
        "the invalid button's fill"
    );
}

#[test]
fn the_submit_label_is_on_primary_in_both_cases() {
    let style = kit().resolved_style();

    let valid = kit();
    let slot = submit_slot(valid);
    let (_, size, ink) = submit_label_painted(&slot);
    assert_eq!(size, style.submit_font_size, "v1's 14");
    assert_eq!(ink, paint(style.submit_ink), "v1's `on_primary`");

    let invalid = kit();
    invalid.set_valid(false);
    let slot = submit_slot(invalid);
    let (_, _, ink) = submit_label_painted(&slot);
    assert_eq!(
        ink,
        paint(style.submit_ink),
        "still `on_primary`: v1's container colour never reached the label"
    );
}

#[test]
fn the_error_messages_are_painted_in_the_error_ink() {
    let style = kit().resolved_style();
    let kit = kit();
    kit.set_errors(["Name is required", "Email is required"]);
    let messages = error_messages_painted(kit);
    assert_eq!(messages.len(), 2, "v1's summary listed both");
    assert_eq!(messages[0].0, "Name is required");
    assert_eq!(messages[0].1, style.error_font_size, "the token's size");
    assert_eq!(messages[0].2, paint(style.error_color), "v1's `error` ink");
}

#[test]
fn the_summary_is_empty_but_the_slot_is_still_there() {
    let content = form_content(kit());
    assert_eq!(
        content.children().len(),
        3,
        "v1's column always held three things"
    );
    let summary = content.children().remove(1).build();
    assert_eq!(summary.children().len(), 0, "no messages to paint");
}

#[test]
fn the_callers_content_is_mounted_in_the_first_slot() {
    let child = Child::new(ChildKey::index(0), LayoutRequest::new(), || {
        Box::new(floom_widgets::Text::new("a field"))
    });
    let content = form_content(kit().child(child));
    assert_eq!(content.children().len(), 3, "v1's column held three things");
    let first = content.children().remove(0).build();
    let ops = ops_of(&*first);
    let text = ops.iter().find_map(|op| match op {
        PaintOp::Text { text, .. } => Some(text.clone()),
        _ => None,
    });
    assert_eq!(text.as_deref(), Some("a field"), "the caller's content");
}

#[test]
fn the_submit_callback_fires_while_the_form_is_valid() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(0));
    let s = seen.clone();
    let kit = kit().on_submit(move || {
        *s.lock().unwrap() += 1;
    });
    let slot = submit_slot(kit);

    tap_submit(&slot);
    assert_eq!(*seen.lock().unwrap(), 1, "v1's callback fired");
}

#[test]
fn the_submit_button_does_not_answer_while_the_form_is_invalid() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(0));
    let s = seen.clone();
    let kit = kit().on_submit(move || {
        *s.lock().unwrap() += 1;
    });
    kit.set_valid(false);
    let slot = submit_slot(kit);

    tap_submit(&slot);
    assert_eq!(
        *seen.lock().unwrap(),
        0,
        "v1's handler was guarded by `if valid.get()`"
    );
}

#[test]
fn the_validity_is_the_kit_s_own_state() {
    let kit = kit();
    assert!(kit.is_valid(), "v1's form started valid");
    kit.set_valid(false);
    assert!(!kit.is_valid());
    kit.set_valid(true);
    assert!(kit.is_valid());
}

#[test]
fn the_errors_are_read_back_in_order() {
    let kit = kit();
    kit.set_errors(["First", "Second"]);
    assert_eq!(
        kit.errors(),
        vec!["First".to_string(), "Second".to_string()],
        "the shared map keeps the order"
    );
    kit.set_errors(["Only"]);
    assert_eq!(
        kit.errors(),
        vec!["Only".to_string()],
        "and writing again replaces what was there"
    );
}

#[test]
fn the_submit_button_can_be_left_out() {
    let kit = kit().show_submit(false);
    let content = form_content(kit);
    assert_eq!(
        content.children().len(),
        3,
        "v1's column always held three things"
    );
    let slot = content.children().remove(2).build();
    assert_eq!(
        slot.children().len(),
        0,
        "v1's `Empty::new()`: no button at all"
    );
}

#[test]
fn the_props_report_the_shape_and_not_the_callback() {
    let kit = kit().submit_label("Send").show_submit(false);
    assert_eq!(kit.props().submit_label, "Send");
    assert!(!kit.props().show_submit);
}

#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    assert_ne!(
        FluentForm::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent form took a snapshot, not the app-wide signal"
    );
}
