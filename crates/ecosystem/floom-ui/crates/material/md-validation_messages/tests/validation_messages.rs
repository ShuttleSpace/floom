
use floom_core::{ElementId, Rect, Widget};
use floom_types::Color;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_ui_composites::ValidationMessagesList;
use floom_widget_md_validation_messages::{
    MaterialValidationMessages, MaterialValidationMessagesStyle,
};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 300.0,
    y1: 120.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialValidationMessages {
    MaterialValidationMessages::new().with_theme(theme())
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
fn message_painted(list: &ValidationMessagesList, index: usize) -> (String, f32, Color) {
    let message = list.children()[index].build();
    ops_of(&*message)
        .iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("a message is painted")
}


/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialValidationMessagesStyle = kit().resolved_style();
    let tokens = match theme()
        .component(floom_core_theme::component_tokens::ComponentId::ValidationMessages)
    {
        floom_core_theme::component_tokens::ComponentTokens::ValidationMessages(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.font_size, tokens.font_size, "the token's size");
    assert_eq!(style.gap, tokens.gap, "the token's gap");
    assert_eq!(
        style.error_color,
        theme().colors().error,
        "the theme's error"
    );
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
    assert_eq!(kit().resolved_style().design_system, "material");
}


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

#[test]
fn the_gap_between_the_messages_is_the_one_v1_used() {
    let resolved = kit().resolved_style();
    let list = kit().messages(vec!["One".into()]).view();
    let request = list.layout_request().expect("the list is a column");
    assert_eq!(request.axis, Some(floom_core::Axis::Vertical));
    assert_eq!(request.gap, Some(resolved.painted_gap));
}


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

#[test]
fn the_caller_can_push_into_the_list() {
    let kit = kit();
    let handle = kit.messages_list();
    let list = kit.view();

    assert!(list.children().is_empty(), "nothing to begin with");

    handle.lock().unwrap().push("Name is required".to_string());
    assert_eq!(list.children().len(), 1, "the message is there");
    assert_eq!(
        list.current_messages(),
        vec!["Name is required".to_string()],
        "and the caller can read it back"
    );

    handle.lock().unwrap().push("Name is too short".to_string());
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

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialValidationMessages::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material validation messages read the app-wide signal, not a snapshot"
    );
}
