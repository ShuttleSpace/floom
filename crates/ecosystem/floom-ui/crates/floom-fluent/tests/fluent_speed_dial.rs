#![cfg(feature = "speed_dial")]
//! The ported Fluent speed dial -- a floating action button and a fan of

use std::sync::{Arc, Mutex};

use floom_core::{ElementId, Point, PointerEvent, Rect, Widget, WidgetAny};
use floom_types::Color;
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};
use floom_widgets::Stack;

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::speed_dial::SpeedDialDirection;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_fluent::widgets::speed_dial::{FluentSpeedDial, FluentSpeedDialStyle};

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

fn kit() -> FluentSpeedDial {
    FluentSpeedDial::new().with_theme(theme())
}

fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn fan_buttons(dial: &Stack, fan_index: usize) -> Vec<Box<dyn WidgetAny>> {
    let mut fan = dial.children().remove(fan_index).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = fan.create(&mut create);
    let mut cx = RecordingUpdateContext::new();
    fan.update(&mut cx, id);
    let mut children = fan.children();
    if children.is_empty() {
        return Vec::new();
    }
    let buttons = children.remove(0).build();
    buttons
        .children()
        .into_iter()
        .map(|child| child.build())
        .collect()
}

fn fab_fill(dial: &Stack, fab_index: usize) -> Option<Color> {
    let pressable = dial.children().remove(fab_index).build();
    let surface = pressable.children().remove(0).build();
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

fn fab_icon_ink(dial: &Stack, fab_index: usize) -> Option<Color> {
    let pressable = dial.children().remove(fab_index).build();
    let surface = pressable.children().remove(0).build();
    let icon = surface.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = icon.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    icon.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().iter().find_map(|op| match op {
        PaintOp::Path { fill, .. } => *fill,
        _ => None,
    })
}

fn tap(dial: &Stack, index: usize) {
    let mut pressable = dial.children().remove(index).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = pressable.create(&mut create);
    let at = Point::new(10.0, 10.0);
    let mut down = TestEventContext::new(id, BOX, PointerEvent::primary_down(at).into());
    pressable.event(&mut down, id);
    let mut up = TestEventContext::new(id, BOX, PointerEvent::primary_up(at).into());
    pressable.event(&mut up, id);
}

#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: FluentSpeedDialStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::SpeedDial)
    {
        floom_core_theme::component_tokens::ComponentTokens::SpeedDial(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.fab_size, tokens.fab_size, "the token's button size");
    assert_eq!(
        style.action_size, tokens.action_size,
        "the token's action size"
    );
    assert_eq!(style.gap, tokens.gap, "the token's gap");
    assert_eq!(style.radius, tokens.radius, "the token's radius");
    assert_eq!(
        style.animation_ms, tokens.animation_ms,
        "the token's animation -- a number v1 never used"
    );
    assert_eq!(
        style.fab_background,
        theme().colors().primary,
        "the button's fill"
    );
    assert_eq!(
        style.fab_foreground,
        theme().colors().on_primary,
        "the button's ink"
    );
    assert_eq!(
        style.action_background,
        theme().colors().secondary_container,
        "an action button's fill"
    );
    assert_eq!(
        style.action_foreground,
        theme().colors().on_secondary_container,
        "an action button's ink"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}

#[test]
fn a_disabled_dial_is_v1_s_dimmed_pair() {
    let style = kit().disabled(true).resolved_style();
    let dim = floom_core_theme::tokens::color_ext::with_alpha(theme().colors().on_surface, 12);
    assert_eq!(style.fab_background, dim, "the fill: `on_surface` at 12");
    assert_eq!(
        style.fab_foreground,
        floom_core_theme::tokens::color_ext::with_alpha(theme().colors().on_surface, 38),
        "the ink: `on_surface` at 38"
    );
}

#[test]
fn the_button_is_painted_in_v1_s_ink() {
    let resolved = kit().resolved_style();
    let dial = kit().view();
    assert_eq!(
        fab_fill(&dial, 1),
        Some(paint(resolved.fab_background)),
        "the button's fill"
    );
    assert_eq!(
        fab_icon_ink(&dial, 1),
        Some(paint(resolved.fab_foreground)),
        "and the icon's ink"
    );
}

#[test]
fn the_button_is_the_size_the_token_says() {
    let resolved = kit().resolved_style();
    let dial = kit().view();
    let pressable = dial.children().remove(1).build();
    let surface = pressable.children().remove(0).build();
    let request = surface
        .layout_request()
        .expect("the button asks for its own space");
    assert_eq!(
        request.width,
        Some(floom_core::Length::Px(resolved.fab_size)),
        "the token's button size"
    );
}

#[test]
fn the_fan_is_mounted_only_while_the_dial_is_open() {
    let closed = kit().action("edit", None).action("delete", None).view();
    assert_eq!(
        fan_buttons(&closed, 0).len(),
        0,
        "a closed dial has no fan, as v1's `Empty` view did"
    );

    let open = kit()
        .open(true)
        .action("edit", None)
        .action("delete", None)
        .view();
    assert_eq!(fan_buttons(&open, 0).len(), 2, "an open dial has both");
}

#[test]
fn tapping_the_button_opens_the_dial() {
    let dial = kit().action("edit", None).view();
    tap(&dial, 1);
    assert_eq!(
        fan_buttons(&dial, 0).len(),
        1,
        "the button's tap opened the fan"
    );
}

#[test]
fn a_disabled_dial_does_not_open() {
    let dial = kit().disabled(true).action("edit", None).view();
    tap(&dial, 1);
    assert_eq!(
        fan_buttons(&dial, 0).len(),
        0,
        "v1's disabled button did not toggle the fan"
    );
}

#[test]
fn an_action_reports_its_index() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let dial = kit()
        .open(true)
        .action("edit", None)
        .action("delete", None)
        .on_action(move |index| recorder.lock().unwrap().push(index))
        .view();

    let mut buttons = fan_buttons(&dial, 0);
    let mut second = buttons.remove(1);
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = second.create(&mut create);
    let at = Point::new(10.0, 10.0);
    let mut down = TestEventContext::new(id, BOX, PointerEvent::primary_down(at).into());
    second.event(&mut down, id);
    let mut up = TestEventContext::new(id, BOX, PointerEvent::primary_up(at).into());
    second.event(&mut up, id);

    assert_eq!(seen.lock().unwrap().as_slice(), &[1], "the action's index");
}

#[test]
fn the_direction_decides_the_order() {
    let up = kit()
        .direction(SpeedDialDirection::Up)
        .open(true)
        .action("edit", None)
        .view();
    assert_eq!(fan_buttons(&up, 0).len(), 1, "the fan is first");

    let down = kit()
        .direction(SpeedDialDirection::Down)
        .open(true)
        .action("edit", None)
        .view();
    assert_eq!(fan_buttons(&down, 1).len(), 1, "the fan is second");
}

#[test]
fn the_props_report_the_direction() {
    let kit = kit()
        .direction(SpeedDialDirection::Left)
        .fab_icon("edit")
        .disabled(true);
    assert_eq!(kit.props().direction, SpeedDialDirection::Left);
    assert_eq!(kit.props().fab_icon, "edit");
    assert!(kit.props().disabled);
}

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_ne!(
        FluentSpeedDial::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent speed dial took a snapshot, not the app-wide signal"
    );
}
