#![cfg(feature = "navigation_drawer")]
//! The ported Fluent navigation drawer -- a panel that slides in from the

use floom_core::{ElementId, Event, Point, PointerEvent, Rect, Widget};
use floom_types::Color;
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::navigation_drawer::NavigationDrawerSide;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_fluent::widgets::navigation_drawer::{
    FluentNavigationDrawer, FluentNavigationDrawerStyle,
};

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

fn kit() -> FluentNavigationDrawer {
    FluentNavigationDrawer::new().with_theme(theme())
}

fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn presence(kit: FluentNavigationDrawer) -> Box<dyn floom_core::WidgetAny> {
    let view = kit.view();
    let mut presence = view.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = presence.create(&mut create);
    let mut cx = RecordingUpdateContext::new();
    presence.update(&mut cx, id);
    presence
}

fn layer(kit: FluentNavigationDrawer) -> Box<dyn floom_core::WidgetAny> {
    let presence = presence(kit);
    presence.children().remove(0).build()
}

fn row(kit: FluentNavigationDrawer) -> Box<dyn floom_core::WidgetAny> {
    let layer = layer(kit);
    layer.children().remove(0).build()
}

fn ops_of(widget: &dyn Widget) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: FluentNavigationDrawerStyle = kit().resolved_style();
    let tokens = match theme()
        .component(floom_core_theme::component_tokens::ComponentId::NavigationDrawer)
    {
        floom_core_theme::component_tokens::ComponentTokens::NavigationDrawer(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.padding, tokens.padding, "the token's padding");
    assert_eq!(
        style.background,
        theme().colors().surface,
        "the panel's fill"
    );
    assert_eq!(style.scrim, theme().colors().scrim, "the scrim's colour");
    assert_eq!(style.scrim_alpha, 160, "v1's scrim alpha");
    assert_eq!(style.accent_width, 3.0, "v1's accent width");
    assert_eq!(style.accent_color, theme().colors().primary, "v1's accent");
    assert_eq!(style.elevation, 4, "v1's default elevation");
    assert_eq!(style.shadow_blur, 3.0 * 4.0, "v1's shadow blur");
    assert_eq!(style.shadow_spread, 1.0, "v1's shadow spread");
}

#[test]
fn the_width_is_the_prop_s_and_there_is_no_token_fallback() {
    let style = kit().resolved_style();
    assert_eq!(style.width, kit().props().width, "the prop's width wins");
    let kit = kit().width(0.0);
    assert_eq!(
        kit.resolved_style().width,
        0.0,
        "v1 took the prop's width and never asked the token"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}

#[test]
fn the_layer_is_painted_the_scrim_colour() {
    let resolved = kit().resolved_style();
    let layer = layer(kit().open(true));
    let fill = ops_of(&*layer).iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    let expected = paint(resolved.scrim);
    let expected = Color {
        a: resolved.scrim_alpha,
        ..expected
    };
    assert_eq!(fill, Some(expected), "the scrim's fill");
}

#[test]
fn the_panel_is_painted_v1_s_colour() {
    let resolved = kit().resolved_style();
    let row = row(kit().open(true));
    let panel = row.children().remove(0).build();
    let fill = ops_of(&*panel).iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(fill, Some(paint(resolved.background)), "the panel's fill");
}

#[test]
fn the_accent_is_a_band_on_the_edge_the_drawer_is_not_on() {
    let resolved = kit().resolved_style();
    let row = row(kit().open(true).side(NavigationDrawerSide::Left));
    let panel = row.children().remove(0).build();
    let band = ops_of(&*panel).iter().find_map(|op| match op {
        PaintOp::Rect { bounds, color } if bounds.width() < 10.0 => Some((*bounds, *color)),
        _ => None,
    });
    let (bounds, color) = band.expect("v1's accent is a band");
    assert_eq!(bounds.width(), 3.0, "v1's accent width");
    assert_eq!(color, paint(resolved.accent_color), "v1's accent colour");
}

#[test]
fn the_row_is_the_panel_and_the_scrim() {
    let left = row(kit().open(true).side(NavigationDrawerSide::Left));
    assert_eq!(left.children().len(), 2, "the panel and the scrim");

    let right = row(kit().open(true).side(NavigationDrawerSide::Right));
    assert_eq!(right.children().len(), 2, "the scrim and the panel");
}

#[test]
fn a_closed_drawer_mounts_nothing() {
    let presence = presence(kit());
    assert!(
        presence.children().is_empty(),
        "v1 hid the whole overlay while the drawer was closed"
    );
}

#[test]
fn the_scrim_closes_the_drawer_when_it_is_clicked() {
    let kit = kit().open(true);
    let open = kit.open_flag();
    assert!(floom_widgets::read_flag(&open), "the drawer is open");
    let row = row(kit);
    let mut scrim = row.children().remove(1).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = scrim.create(&mut create);
    let mut down = TestEventContext::new(
        id,
        BOX,
        Event::Pointer(PointerEvent::primary_down(Point::new(10.0, 10.0))),
    );
    scrim.event(&mut down, id);
    let mut up = TestEventContext::new(
        id,
        BOX,
        Event::Pointer(PointerEvent::primary_up(Point::new(10.0, 10.0))),
    );
    scrim.event(&mut up, id);
    assert!(
        !floom_widgets::read_flag(&open),
        "v1's scrim closed the drawer when it was clicked"
    );
}

#[test]
fn the_props_report_the_side_and_the_width() {
    let kit = kit().side(NavigationDrawerSide::Right).width(200.0);
    assert_eq!(kit.props().side, NavigationDrawerSide::Right);
    assert_eq!(kit.props().width, 200.0);
}

#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    assert_ne!(
        FluentNavigationDrawer::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent drawer took a snapshot, not the app-wide signal"
    );
}
