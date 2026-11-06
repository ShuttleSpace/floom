use std::sync::{Arc, Mutex};

use floom_core::{ElementId, Event, Rect, Widget, WidgetAny};
use floom_types::{Color, Key, KeyEvent};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_md_otp_input::{MaterialOtpInput, MaterialOtpInputStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialOtpInput {
    MaterialOtpInput::new().with_theme(theme())
}

/// Theme colour -> engine colour.
fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// One widget's paint ops.
fn ops_of<W: Widget + ?Sized>(widget: &W) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

/// A character typed into a box.
fn type_text<W: Widget + ?Sized>(widget: &mut W, ch: char) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut key = TestEventContext::new(id, BOX, Event::Key(KeyEvent::down(Key::Char(ch))));
    widget.event(&mut key, id);
}

/// The text a box paints.
fn painted_text<W: Widget + ?Sized>(widget: &W) -> Option<(String, f32, Color)> {
    ops_of(widget).into_iter().find_map(|op| match op {
        PaintOp::Text {
            text, size, color, ..
        } => Some((text, size, color)),
        _ => None,
    })
}

/// The text input inside a box.
fn input_of(box_widget: &dyn WidgetAny) -> Box<dyn WidgetAny> {
    box_widget.children().remove(0).build()
}

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialOtpInputStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::OtpInput)
    {
        floom_core_theme::component_tokens::ComponentTokens::OtpInput(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.box_size, tokens.box_size, "the token's box size");
    assert_eq!(
        style.border_radius, tokens.border_radius,
        "the token's radius"
    );
    assert_eq!(style.gap, tokens.gap, "the token's gap");
    assert_eq!(style.font_size, tokens.font_size, "the token's font size");
    assert_eq!(style.text_color, theme().colors().on_surface, "the ink");
    assert_eq!(style.background, theme().colors().surface, "the box's fill");
    assert_eq!(
        style.border_color,
        theme().colors().outline,
        "the box's rim"
    );
    assert_eq!(style.border_width, 1.0, "v1's rim width");
    assert_eq!(style.length, 6, "v1's default length");
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

#[test]
fn the_boxes_are_painted_in_v1_s_colours() {
    let resolved = kit().length(3).resolved_style();
    let view = kit().length(3).view();
    let mut children = view.children();
    assert_eq!(children.len(), 3, "one box per character");

    let box_widget = children.remove(0).build();
    let ops = ops_of(&*box_widget);
    let fill = ops.iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(fill, Some(paint(resolved.background)), "the box's fill");

    let rim = ops.iter().find_map(|op| match op {
        PaintOp::Path {
            stroke: Some((_, color)),
            ..
        } => Some(*color),
        _ => None,
    });
    assert_eq!(rim, Some(paint(resolved.border_color)), "the box's rim");

    let request = box_widget
        .layout_request()
        .expect("the box asks for its own space");
    assert_eq!(
        request.width,
        Some(floom_core::Length::Px(resolved.box_size)),
        "the box's width"
    );
    assert_eq!(
        request.height,
        Some(floom_core::Length::Px(resolved.box_size)),
        "the box's height"
    );
}

#[test]
fn the_character_is_painted_in_v1_s_ink() {
    let resolved = kit().length(3).resolved_style();
    let view = kit().length(3).view();
    let mut children = view.children();
    let box_widget = children.remove(0).build();
    let mut input = input_of(&*box_widget);
    type_text(&mut *input, '5');

    let (text, size, ink) = painted_text(&*input).expect("the character is painted");
    assert_eq!(text, "5");
    assert_eq!(size, resolved.font_size, "v1's font size");
    assert_eq!(ink, paint(resolved.text_color), "v1's ink");
}

/// A masked box shows a dot, and the real character stays in the code.
#[test]
fn a_masked_box_shows_a_dot() {
    let kit = kit().length(3).masked(true);
    let code = kit.code();
    let view = kit.view();
    let mut children = view.children();
    let box_widget = children.remove(0).build();
    let mut input = input_of(&*box_widget);
    type_text(&mut *input, '5');

    let (text, _, _) = painted_text(&*input).expect("the mask is painted");
    assert_eq!(text, "\u{2022}", "the box shows the mask");
    assert_eq!(
        *code.lock().expect("the code is not poisoned"),
        "5",
        "and the code keeps the real character"
    );
}

/// The gap between the boxes is the token's.
#[test]
fn the_gap_between_the_boxes_is_the_token_s_gap() {
    let resolved = kit().resolved_style();
    let view = kit().view();
    let request = view
        .layout_request()
        .expect("the row asks for its own space");
    assert_eq!(request.gap, Some(resolved.gap), "the token's gap");
}

/// Typing in a box writes the code and fires the callback.
#[test]
fn typing_writes_the_code() {
    let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let kit = kit().length(3).on_change(move |code| {
        recorder
            .lock()
            .expect("the recorder is not poisoned")
            .push(code);
    });
    let code = kit.code();
    let view = kit.view();

    let mut children = view.children();
    let box_widget = children.remove(0).build();
    let mut input = input_of(&*box_widget);
    type_text(&mut *input, '7');

    assert_eq!(
        *code.lock().expect("the code is not poisoned"),
        "7",
        "the box's character is the code"
    );
    assert_eq!(
        *seen.lock().expect("the recorder is not poisoned"),
        vec!["7".to_string()],
        "and the callback heard it"
    );
}

#[test]
fn the_code_skips_the_empty_boxes() {
    let kit = kit().length(3);
    let code = kit.code();
    let view = kit.view();
    let mut children = view.children();
    let second = children.remove(1).build();
    let mut input = input_of(&*second);
    type_text(&mut *input, '4');
    assert_eq!(
        *code.lock().expect("the code is not poisoned"),
        "4",
        "the empty first box is left out"
    );
}

/// The writer pushes the code into the boxes.
#[test]
fn the_writer_pushes_the_code_into_the_boxes() {
    let kit = kit().length(3);
    let code = kit.code();
    let writer = kit.code_writer();
    let view = kit.view();

    writer("123");

    assert_eq!(
        *code.lock().expect("the code is not poisoned"),
        "123",
        "the code was written"
    );

    let mut children = view.children();
    let box_widget = children.remove(0).build();
    let input = input_of(&*box_widget);
    let (text, _, _) = painted_text(&*input).expect("the box paints its character");
    assert_eq!(text, "1", "and the first box shows the first character");
}

/// A disabled input does not move the code.
#[test]
fn a_disabled_input_does_not_move_the_code() {
    let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let kit = kit().length(3).disabled(true).on_change(move |code| {
        recorder
            .lock()
            .expect("the recorder is not poisoned")
            .push(code);
    });
    let code = kit.code();
    let view = kit.view();

    let mut children = view.children();
    let box_widget = children.remove(0).build();
    let mut input = input_of(&*box_widget);
    type_text(&mut *input, '7');

    assert_eq!(
        *code.lock().expect("the code is not poisoned"),
        "",
        "a disabled input does not move the code"
    );
    assert!(
        seen.lock()
            .expect("the recorder is not poisoned")
            .is_empty(),
        "and does not answer"
    );
}

/// The props report the shape, and the shared props' own callback slot stays
/// empty: the kit keeps a shareable copy of its own.
#[test]
fn the_props_report_the_shape() {
    let kit = kit().length(4).masked(true).disabled(true);
    let props = kit.props();
    assert_eq!(props.length, 4);
    assert!(props.masked);
    assert!(props.disabled);
    assert!(
        props.on_change.is_none(),
        "the shared props' callback is not shareable, so the kit keeps its own"
    );
}

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialOtpInput::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material OTP input read the app-wide signal, not a snapshot"
    );
}
