//! The ported Material textarea -- a bordered box with a text field in it,
//! sized by the rows.
//!
//! The shape is the engine's vocabulary plus the engine's `TextInput`, so what
//! is tested here is the translation and the pixels: the tokens reach the
//! style, the box is painted in v1's colours, the text is painted at the size
//! v1 painted it, and the dead numbers v1 wrote are still the ones the style
//! reports.
//!
//! ## The tripwires
//!
//! `the_box_is_painted_in_v1_s_colours` and `the_dead_numbers_are_v1_s` would
//! fail if this kit stopped being this kit.

use std::sync::{Arc, Mutex};

use floom_core::{ElementId, Event, Rect, Widget};
use floom_types::{Color, Key, KeyEvent};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_md_textarea::{MaterialTextarea, MaterialTextareaStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    // this test's alone.
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialTextarea {
    MaterialTextarea::new().with_theme(theme())
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

/// A character typed into the field.
fn type_text<W: Widget + ?Sized>(widget: &mut W, ch: char) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut key = TestEventContext::new(id, BOX, Event::Key(KeyEvent::down(Key::Char(ch))));
    widget.event(&mut key, id);
}

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialTextareaStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Textarea)
    {
        floom_core_theme::component_tokens::ComponentTokens::Textarea(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.padding, tokens.padding, "the token's padding");
    assert_eq!(
        style.border_radius, tokens.border_radius,
        "the token's radius"
    );
    assert_eq!(style.min_height, tokens.min_height, "the token's height");
    assert_eq!(style.font_size, tokens.font_size, "the token's font size");
    assert_eq!(
        style.max_width, tokens.max_width,
        "the token's maximum width"
    );
    assert_eq!(
        style.text_font_size,
        theme().typography().body.medium.font_size,
        "the size the text is painted at"
    );
    assert_eq!(style.background, theme().colors().surface, "the box's fill");
    assert_eq!(
        style.border_color,
        theme().colors().outline,
        "the box's rim"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

/// The numbers v1 wrote and never drew are still the ones the style reports:
/// the disabled ink at 97, and the token's font size rather than the size the
/// text is painted at.
#[test]
fn the_dead_numbers_are_v1_s() {
    let style = kit().disabled(true).resolved_style();
    let palette = theme();
    let colors = palette.colors();
    assert_eq!(
        style.text_color,
        floom_core_theme::tokens::color::Color {
            a: 97,
            ..colors.on_surface
        },
        "v1's style said 97 while its view painted 38"
    );
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Textarea)
    {
        floom_core_theme::component_tokens::ComponentTokens::Textarea(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(
        style.font_size, tokens.font_size,
        "v1's style reported the token's size, not the one it painted at"
    );
}

// --- the pixels -----------------------------------------------------------

/// The box is painted in v1's colours.
#[test]
fn the_box_is_painted_in_v1_s_colours() {
    let resolved = kit().resolved_style();
    let view = kit().view();
    let ops = ops_of(&view);

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
}

/// The text is painted in v1's ink, at the size v1 painted it.
#[test]
fn the_text_is_painted_in_v1_s_ink() {
    let resolved = kit().resolved_style();
    let view = kit().value("Hello").view();
    let field = view.children().remove(0).build();
    let ops = ops_of(&*field);

    let (text, size, ink) = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("the text is painted");
    assert_eq!(text, "Hello");
    assert_eq!(
        size, resolved.text_font_size,
        "the size v1's view used, not the token's"
    );
    assert_eq!(ink, paint(theme().colors().on_surface), "v1's ink");
}

/// The box is as tall as the rows ask for.
#[test]
fn the_box_is_as_tall_as_the_rows_ask() {
    let style = kit().rows(4).resolved_style();
    let view = kit().rows(4).view();
    let field = view.children().remove(0);
    let expected = style.min_height + 3.0 * style.line_height;
    assert_eq!(
        field.layout.height,
        Some(floom_core::Length::Px(expected)),
        "the field is as tall as the rows ask for"
    );
}

// --- the structure --------------------------------------------------------

/// Typing fires the callback, and the text is the field's.
#[test]
fn typing_fires_the_callback() {
    let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let kit = kit().on_change(move |text| {
        recorder
            .lock()
            .expect("the recorder is not poisoned")
            .push(text);
    });
    let view = kit.view();
    let field = view.children().remove(0).build();
    let mut input = field;
    type_text(&mut *input, 'a');

    assert_eq!(
        *seen.lock().expect("the recorder is not poisoned"),
        vec!["a".to_string()],
        "the callback heard the edit"
    );
}

/// A disabled field does not answer.
#[test]
fn a_disabled_field_does_not_answer() {
    let seen: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let kit = kit().disabled(true).on_change(move |text| {
        recorder
            .lock()
            .expect("the recorder is not poisoned")
            .push(text);
    });
    let view = kit.view();
    let field = view.children().remove(0).build();
    let mut input = field;
    type_text(&mut *input, 'a');

    assert!(
        seen.lock()
            .expect("the recorder is not poisoned")
            .is_empty(),
        "a disabled field does not answer"
    );
}

/// The text is a shared buffer the host can read and write.
#[test]
fn the_text_is_a_shared_buffer() {
    let kit = kit().value("Hi");
    let buffer = kit.buffer();
    assert_eq!(kit.text(), "Hi");
    assert_eq!(
        *buffer.lock().expect("the buffer is not poisoned"),
        "Hi",
        "and the buffer holds it"
    );
}

/// The props report the text, and the shared props' own callback slot stays
/// empty: the kit keeps a shareable copy of its own.
#[test]
fn the_props_report_the_text() {
    let kit = kit().value("Hi").placeholder("Type").rows(5).disabled(true);
    let props = kit.props();
    assert_eq!(props.value, "Hi");
    assert_eq!(props.placeholder, "Type");
    assert_eq!(props.rows, 5);
    assert!(props.disabled);
    assert!(
        props.on_change.is_none(),
        "the shared props' callback is not shareable, so the kit keeps its own"
    );
}

/// The theme: v1's Material textarea read the *app-wide* theme signal, so a
/// change of theme re-themes it with everything else.
#[test]
fn the_theme_is_the_app_wide_one() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    // exactly the thing this test is about.
    assert_eq!(
        MaterialTextarea::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material textarea read the app-wide signal, not a snapshot"
    );
}
