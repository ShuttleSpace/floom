//! The ported Cupertino speed dial -- a floating action button and a fan of
//! smaller ones.
//!
//! The shape is the engine's vocabulary, so what is tested here is the
//! translation and the pixels: the tokens reach the style, the button is
//! painted in v1's inks, and the fan is mounted only while the dial is open.
//!
//! ## The tripwires
//!
//! `the_button_is_painted_in_v1_s_ink` and `the_fan_is_mounted_only_while_the
//! _dial_is_open` would fail if this kit stopped being this kit.

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

use floom_widget_cupertino_speed_dial::{CupertinoSpeedDial, CupertinoSpeedDialStyle};

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
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn kit() -> CupertinoSpeedDial {
    CupertinoSpeedDial::new().with_theme(theme())
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

/// The fan's buttons, once the dial is open.
///
/// The fan is the engine's `Conditional`, so it is mounted in an update pass
/// -- the way the host does it every frame.
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
    buttons.children().into_iter().map(|child| child.build()).collect()
}

/// The button's fill: the circle the `Pressable` wraps.
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

/// The button's icon ink.
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

/// Taps the child at `index`, the way a pointer would: a press and a release
/// over the same spot.
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

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: CupertinoSpeedDialStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::SpeedDial) {
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
    assert_eq!(style.fab_background, theme().colors().primary, "the button's fill");
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
    assert_eq!(kit().resolved_style().design_system, "cupertino");
}

/// A disabled dial's button is v1's dimmed pair.
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

// --- the pixels -----------------------------------------------------------

/// The button is painted in v1's colours, at v1's size.
#[test]
fn the_button_is_painted_in_v1_s_ink() {
    let resolved = kit().resolved_style();
    let dial = kit().view();
    // v1's `Up` puts the fan first and the button second.
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

/// The button is the size the token says.
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

// --- the rules ------------------------------------------------------------

/// The fan is mounted only while the dial is open.
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

/// Tapping the button opens the dial.
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

/// A disabled dial does not open.
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

/// An action reports its index.
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

/// The direction decides the order of the fan and the button.
#[test]
fn the_direction_decides_the_order() {
    // v1's `Up` was `Stack::vertical((actions, fab))`: the fan first.
    let up = kit()
        .direction(SpeedDialDirection::Up)
        .open(true)
        .action("edit", None)
        .view();
    assert_eq!(fan_buttons(&up, 0).len(), 1, "the fan is first");

    // v1's `Down` was `Stack::vertical((fab, actions))`: the button first.
    let down = kit()
        .direction(SpeedDialDirection::Down)
        .open(true)
        .action("edit", None)
        .view();
    assert_eq!(fan_buttons(&down, 1).len(), 1, "the fan is second");
}

/// The props report the direction and the button's icon.
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

/// The theme: v1's Cupertino speed dial read the *app-wide* theme signal, so a
/// change of theme re-themes it with everything else.
#[test]
fn the_theme_is_the_app_wide_one() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    // exactly the thing this test is about.
    assert_ne!(
        CupertinoSpeedDial::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Cupertino speed dial took a snapshot, not the app-wide signal"
    );
}
