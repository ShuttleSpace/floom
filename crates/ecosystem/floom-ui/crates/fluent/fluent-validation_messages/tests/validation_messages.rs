//! The ported Fluent validation messages -- a column of error strings.
//!
//! The list is the kit's own, because the engine has no shared list (see the
//! kit's module documentation), so what is tested here is the translation and
//! the pixels: the tokens reach the style, the messages are painted in the
//! theme's error colour at the size v1 drew them, and the caller can push into
//! the list and pull out of it.
//!
//! ## The tripwires
//!
//! `the_messages_are_painted_in_the_error_colour` and
//! `the_caller_can_push_into_the_list` would fail if this kit stopped being
//! this kit.

use floom_core::{ElementId, Rect, Widget};
use floom_types::Color;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_validation_messages::{
    FluentValidationMessages, FluentValidationMessagesList, FluentValidationMessagesStyle,
};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 300.0,
    y1: 120.0,
};

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    // this test's alone.
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentValidationMessages {
    FluentValidationMessages::new().with_theme(theme())
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
fn ops_of(widget: &dyn Widget) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

/// The text, the size and the ink one message painted.
fn message_painted(list: &FluentValidationMessagesList, index: usize) -> (String, f32, Color) {
    let message = list.children()[index].build();
    ops_of(&*message)
        .iter()
        .find_map(|op| match op {
            PaintOp::Text { text, size, color, .. } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("a message is painted")
}

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: FluentValidationMessagesStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::ValidationMessages) {
        floom_core_theme::component_tokens::ComponentTokens::ValidationMessages(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.font_size, tokens.font_size, "the token's size");
    assert_eq!(style.gap, tokens.gap, "the token's gap");
    assert_eq!(style.error_color, theme().colors().error, "the theme's error");
    // The props default to the tokens, so the two readings agree until a
    // caller overrides one of them.
    assert_eq!(style.painted_font_size, tokens.font_size);
    assert_eq!(style.painted_gap, tokens.gap);
}

/// The props override what is drawn, and the style says so.
#[test]
fn the_props_override_what_is_drawn() {
    let style = kit().gap(10.0).font_size(18.0).resolved_style();
    assert_eq!(style.painted_gap, 10.0, "the prop's gap");
    assert_eq!(style.painted_font_size, 18.0, "the prop's size");
    assert_eq!(style.gap, 4.0, "while v1's field still reports the token");
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}

// --- the pixels -----------------------------------------------------------

/// The messages are painted in the theme's error colour, at the size v1 drew
/// them.
#[test]
fn the_messages_are_painted_in_the_error_colour() {
    let resolved = kit().resolved_style();
    let list = kit()
        .messages(vec!["Name is required".into(), "Name is too short".into()])
        .view();

    let (text, size, ink) = message_painted(&list, 0);
    assert_eq!(text, "Name is required");
    assert_eq!(size, resolved.painted_font_size, "v1's message size");
    assert_eq!(ink, paint(resolved.error_color), "the theme's error colour");

    let (text, _, ink) = message_painted(&list, 1);
    assert_eq!(text, "Name is too short");
    assert_eq!(ink, paint(resolved.error_color), "and the same ink");
}

/// The gap between the messages is the one v1 used.
#[test]
fn the_gap_between_the_messages_is_the_one_v1_used() {
    let resolved = kit().resolved_style();
    let list = kit().messages(vec!["One".into()]).view();
    let request = list.layout_request().expect("the list is a column");
    assert_eq!(request.axis, Some(floom_core::Axis::Vertical));
    assert_eq!(request.gap, Some(resolved.painted_gap));
}

// --- the structure --------------------------------------------------------

/// An empty list is a column with nothing in it: v1's "collapses to a
/// zero-height empty view".
#[test]
fn an_empty_list_has_no_messages() {
    let list = kit().view();
    assert!(list.children().is_empty(), "no messages");
    assert_eq!(
        list.layout_request().and_then(|r| r.axis),
        Some(floom_core::Axis::Vertical),
        "and it is still a column"
    );
}

/// The caller can push into the list and pull out of it: v1's
/// `messages_signal()`.
#[test]
fn the_caller_can_push_into_the_list() {
    let kit = kit();
    let handle = kit.messages_list();
    let list = kit.view();

    assert!(list.children().is_empty(), "nothing to begin with");

    handle
        .lock()
        .unwrap()
        .push("Name is required".to_string());
    assert_eq!(list.children().len(), 1, "the message is there");
    assert_eq!(
        list.current_messages(),
        vec!["Name is required".to_string()],
        "and the caller can read it back"
    );

    // The handle the caller holds is the same list the widget draws.
    handle
        .lock()
        .unwrap()
        .push("Name is too short".to_string());
    assert_eq!(list.children().len(), 2, "two messages");
}

#[test]
fn the_caller_can_take_the_messages_away() {
    let kit = kit().messages(vec!["One".into()]);
    let handle = kit.messages_list();
    let list = kit.view();
    assert_eq!(list.children().len(), 1);

    handle.lock().unwrap().clear();
    assert!(list.children().is_empty(), "and the list is empty again");
}

/// The props report the overrides.
#[test]
fn the_props_report_the_overrides() {
    let kit = kit().gap(8.0).font_size(16.0);
    assert_eq!(kit.props().gap, 8.0);
    assert_eq!(kit.props().font_size, 16.0);
}

/// The theme: v1's Fluent validation messages took a *snapshot* of its own
/// design system's theme, so a change of the app-wide theme does not reach
/// them.
#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    // exactly the thing this test is about.
    assert_ne!(
        FluentValidationMessages::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent validation messages took a snapshot, not the app-wide signal"
    );
}
