#![cfg(feature = "navigation_drawer")]
use floom_core::{ElementId, Event, Point, PointerEvent, Rect, Widget};
use floom_types::Color;
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::navigation_drawer::NavigationDrawerSide;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_material::widgets::navigation_drawer::{
    MaterialNavigationDrawer, MaterialNavigationDrawerStyle,
};

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

fn kit() -> MaterialNavigationDrawer {
    MaterialNavigationDrawer::new().with_theme(theme())
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

/// The layer the drawer lives in: the scrim's presence, then the panel's.
fn layer(kit: MaterialNavigationDrawer) -> Box<dyn floom_core::WidgetAny> {
    let view = kit.view();
    view.children().remove(0).build()
}

/// The scrim's presence, mounted and reconciled so that it holds its child.
fn scrim_presence(kit: MaterialNavigationDrawer) -> Box<dyn floom_core::WidgetAny> {
    let layer = layer(kit);
    let mut presence = layer.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = presence.create(&mut create);
    let mut cx = RecordingUpdateContext::new();
    presence.update(&mut cx, id);
    presence
}

/// The panel's presence, mounted and reconciled so that it holds its child.
fn panel_presence(kit: MaterialNavigationDrawer) -> Box<dyn floom_core::WidgetAny> {
    let layer = layer(kit);
    let mut presence = layer.children().remove(1).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = presence.create(&mut create);
    let mut cx = RecordingUpdateContext::new();
    presence.update(&mut cx, id);
    presence
}

/// The fill of one painted box.
fn fill_of(widget: &dyn Widget) -> Option<Color> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    })
}

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialNavigationDrawerStyle = kit().resolved_style();
    let tokens = match theme()
        .component(floom_core_theme::component_tokens::ComponentId::NavigationDrawer)
    {
        floom_core_theme::component_tokens::ComponentTokens::NavigationDrawer(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.padding, tokens.padding, "the token's padding");
    assert_eq!(
        style.panel_color,
        theme().colors().surface,
        "the panel's fill"
    );
    assert_eq!(
        style.scrim_color,
        theme().colors().scrim,
        "the scrim's colour"
    );
    assert_eq!(style.scrim_alpha, 150, "v1's scrim alpha");
    assert_eq!(style.elevation, 4, "v1's default elevation");
    assert_eq!(style.shadow_blur, 2.0 + 3.0 * 4.0, "v1's shadow blur");
    assert_eq!(style.shadow_spread, 1.0, "v1's shadow spread");
}

/// The width is the prop's, and the token is only the fallback.
#[test]
fn the_width_is_the_prop_s_and_the_token_is_the_fallback() {
    let style = kit().resolved_style();
    assert_eq!(style.width, kit().props().width, "the prop's width wins");
    let kit = kit().width(0.0);
    let tokens = match theme()
        .component(floom_core_theme::component_tokens::ComponentId::NavigationDrawer)
    {
        floom_core_theme::component_tokens::ComponentTokens::NavigationDrawer(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(
        kit.resolved_style().width,
        tokens.width,
        "and the token's is the fallback, which is what v1 wrote"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

#[test]
fn the_panel_is_painted_v1_s_colour() {
    let resolved = kit().resolved_style();
    let presence = panel_presence(kit().open(true));
    let panel = presence.children().remove(0).build();
    assert_eq!(
        fill_of(&*panel),
        Some(paint(resolved.panel_color)),
        "the panel's fill"
    );
}

#[test]
fn the_scrim_is_painted_v1_s_colour() {
    let resolved = kit().resolved_style();
    let presence = scrim_presence(kit().open(true));
    let pressable = presence.children().remove(0).build();
    let scrim = pressable.children().remove(0).build();
    let expected = paint(resolved.scrim_color);
    let expected = Color {
        a: resolved.scrim_alpha,
        ..expected
    };
    assert_eq!(fill_of(&*scrim), Some(expected), "the scrim's fill");
}

/// A closed drawer mounts nothing at all.
#[test]
fn a_closed_drawer_mounts_nothing() {
    let presence = scrim_presence(kit());
    assert!(
        presence.children().is_empty(),
        "v1 hid the whole scrim while the drawer was closed"
    );
}

#[test]
fn the_panel_is_pinned_to_the_chosen_edge() {
    let presence = panel_presence(kit().open(true).side(NavigationDrawerSide::Left));
    let panel = presence.children().remove(0).build();
    let request = panel
        .layout_request()
        .expect("the panel asks for its own box");
    assert_eq!(
        request.position,
        Some(floom_core::Position::Absolute),
        "v1's panel was absolutely positioned"
    );
    let inset = request.inset.expect("v1's panel was inset");
    assert_eq!(inset.left, Some(0.0), "v1 pinned it to the left");
    assert_eq!(inset.right, None, "and not to the right");

    let presence = panel_presence(kit().open(true).side(NavigationDrawerSide::Right));
    let panel = presence.children().remove(0).build();
    let request = panel
        .layout_request()
        .expect("the panel asks for its own box");
    let inset = request.inset.expect("v1's panel was inset");
    assert_eq!(inset.right, Some(0.0), "v1 pinned it to the right");
    assert_eq!(inset.left, None, "and not to the left");
}

/// The scrim closes the drawer when it is clicked.
#[test]
fn the_scrim_closes_the_drawer_when_it_is_clicked() {
    let kit = kit().open(true);
    let open = kit.open_flag();
    assert!(floom_widgets::read_flag(&open), "the drawer is open");
    let presence = scrim_presence(kit);
    let mut pressable = presence.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = pressable.create(&mut create);
    let mut down = TestEventContext::new(
        id,
        BOX,
        Event::Pointer(PointerEvent::primary_down(Point::new(10.0, 10.0))),
    );
    pressable.event(&mut down, id);
    let mut up = TestEventContext::new(
        id,
        BOX,
        Event::Pointer(PointerEvent::primary_up(Point::new(10.0, 10.0))),
    );
    pressable.event(&mut up, id);
    assert!(
        !floom_widgets::read_flag(&open),
        "v1's scrim closed the drawer when it was clicked"
    );
}

/// The props report the side and the width.
#[test]
fn the_props_report_the_side_and_the_width() {
    let kit = kit().side(NavigationDrawerSide::Right).width(200.0);
    assert_eq!(kit.props().side, NavigationDrawerSide::Right);
    assert_eq!(kit.props().width, 200.0);
}

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialNavigationDrawer::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material drawer read the app-wide signal, not a snapshot"
    );
}
