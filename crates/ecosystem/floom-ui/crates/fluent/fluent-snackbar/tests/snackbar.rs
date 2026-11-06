//! The ported Fluent snackbar -- a bar at the bottom of the page with a

use std::sync::{Arc, Mutex};
use std::time::Duration;

use floom_core::{ElementId, Point, PointerEvent, Rect, Widget, WidgetAny};
use floom_types::Color;
use floom_ui_composites::Overlay;
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_snackbar::{FluentSnackbar, FluentSnackbarStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentSnackbar {
    FluentSnackbar::new().with_theme(theme())
}

fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn bar(overlay: &Overlay) -> Option<Box<dyn WidgetAny>> {
    let mut conditional = overlay.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = conditional.create(&mut create);
    let mut cx = RecordingUpdateContext::new();
    conditional.update(&mut cx, id);
    let mut children = conditional.children();
    if children.is_empty() {
        return None;
    }
    let layer = children.remove(0).build();
    let surface = layer.children().remove(0).build();
    Some(surface)
}

fn bar_fill(surface: &Box<dyn WidgetAny>) -> Option<Color> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = surface.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    surface.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    })
}

fn message_ink(surface: &Box<dyn WidgetAny>) -> Option<Color> {
    let row = surface.children().remove(0).build();
    let message = row.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = message.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    message.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().iter().find_map(|op| match op {
        PaintOp::Text { color, .. } => Some(*color),
        _ => None,
    })
}

fn action_ink(surface: &Box<dyn WidgetAny>) -> Option<Color> {
    let row = surface.children().remove(0).build();
    let action = row.children().remove(1).build();
    let stack = action.children().remove(0).build();
    let text = stack.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = text.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    text.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().iter().find_map(|op| match op {
        PaintOp::Text { color, .. } => Some(*color),
        _ => None,
    })
}

fn press_action(surface: &Box<dyn WidgetAny>) {
    let row = surface.children().remove(0).build();
    let mut action = row.children().remove(1).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = action.create(&mut create);
    let at = Point::new(10.0, 10.0);
    let mut down = TestEventContext::new(id, BOX, PointerEvent::primary_down(at).into());
    action.event(&mut down, id);
    let mut up = TestEventContext::new(id, BOX, PointerEvent::primary_up(at).into());
    action.event(&mut up, id);
}

#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: FluentSnackbarStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Snackbar)
    {
        floom_core_theme::component_tokens::ComponentTokens::Snackbar(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.padding, tokens.padding, "the token's padding");
    assert_eq!(
        style.border_radius, tokens.border_radius,
        "the token's radius"
    );
    assert_eq!(style.min_height, tokens.min_height, "the token's height");
    assert_eq!(style.font_size, tokens.font_size, "the token's font size");
    assert_eq!(style.max_width, tokens.max_width, "the token's width");
    assert_eq!(style.action_gap, tokens.action_gap, "the token's gap");
    assert_eq!(
        style.surface_color,
        floom_core_theme::tokens::color_ext::with_alpha(theme().colors().on_surface, 235),
        "the bar's fill: v1's Fluent bar was `on_surface` at 235, not opaque"
    );
    assert_eq!(
        style.text_color,
        theme().colors().surface,
        "the message's ink"
    );
    assert_eq!(
        style.action_color,
        theme().colors().primary,
        "the action's ink"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}

#[test]
fn the_bar_is_painted_in_v1_s_colours() {
    let resolved = kit().resolved_style();
    let overlay = kit()
        .message("Saved")
        .with_open_flag(floom_widgets::shared_flag(true))
        .view();
    let surface = bar(&overlay).expect("the bar is up");
    assert_eq!(
        bar_fill(&surface),
        Some(paint(resolved.surface_color)),
        "the bar's fill"
    );
    assert_eq!(
        message_ink(&surface),
        Some(paint(resolved.text_color)),
        "the message's ink"
    );
}

#[test]
fn the_action_is_painted_in_v1_s_ink() {
    let resolved = kit().resolved_style();
    let overlay = kit()
        .message("Saved")
        .action_label(Some("Undo".into()))
        .with_open_flag(floom_widgets::shared_flag(true))
        .view();
    let surface = bar(&overlay).expect("the bar is up");
    assert_eq!(
        action_ink(&surface),
        Some(paint(resolved.action_color)),
        "the action's ink"
    );
}

#[test]
fn the_bar_is_only_there_while_the_snackbar_is_up() {
    let closed = kit().message("Saved").view();
    assert!(
        bar(&closed).is_none(),
        "a closed snackbar has no bar, as v1's hidden overlay did"
    );

    let open = kit()
        .message("Saved")
        .with_open_flag(floom_widgets::shared_flag(true))
        .view();
    assert!(bar(&open).is_some(), "an open snackbar has its bar");
}

#[test]
fn the_action_closes_the_snackbar() {
    let seen = Arc::new(Mutex::new(false));
    let recorder = seen.clone();
    let show = floom_widgets::shared_flag(true);
    let kit = kit()
        .message("Saved")
        .action_label(Some("Undo".into()))
        .with_open_flag(show.clone())
        .on_action(move || *recorder.lock().unwrap() = true);
    let overlay = kit.view();
    let surface = bar(&overlay).expect("the bar is up");

    press_action(&surface);

    assert!(*seen.lock().unwrap(), "v1 ran the action's callback");
    assert!(
        !floom_widgets::read_flag(&show),
        "and v1 closed the snackbar"
    );
}

#[test]
fn there_is_no_action_without_a_label() {
    let overlay = kit()
        .message("Saved")
        .with_open_flag(floom_widgets::shared_flag(true))
        .view();
    let surface = bar(&overlay).expect("the bar is up");
    let row = surface.children().remove(0).build();
    assert_eq!(row.children().len(), 1, "only the message");
}

#[test]
fn the_dismissal_closes_the_snackbar() {
    let show = floom_widgets::shared_flag(true);
    let kit = kit()
        .message("Saved")
        .duration_ms(20)
        .with_open_flag(show.clone());
    kit.dismiss_after(20);
    std::thread::sleep(Duration::from_millis(250));
    assert!(
        !floom_widgets::read_flag(&show),
        "the snackbar was dismissed"
    );
}

#[test]
fn the_props_report_the_message() {
    let kit = kit().message("Saved").duration_ms(1500);
    assert_eq!(kit.props().message, "Saved");
    assert_eq!(kit.props().duration_ms, 1500);
    assert!(kit.props().action_label.is_none());
}

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_ne!(
        FluentSnackbar::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent snackbar took a snapshot, not the app-wide signal"
    );
}
