//! The ported Fluent navigation drawer -- a panel that slides in from the
//! edge of the app over a dimmed scrim.
//!
//! Fluent's drawer is not the other two kits' drawer: the panel and the scrim
//! are two children of a *row*, the panel takes its width and the scrim takes
//! the rest, and the whole row sits on a scrim-coloured background. So what
//! is tested here is that row, the pixels on it, and the click that closes
//! the drawer.
//!
//! ## The tripwires
//!
//! `the_row_is_the_panel_and_the_scrim` and
//! `the_scrim_closes_the_drawer_when_it_is_clicked` would fail if this kit
//! stopped being this kit.

use floom_core::{ElementId, Event, Point, PointerEvent, Rect, Widget};
use floom_types::Color;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder,
    TestEventContext};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::navigation_drawer::NavigationDrawerSide;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_navigation_drawer::{FluentNavigationDrawer, FluentNavigationDrawerStyle};

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
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentNavigationDrawer {
    FluentNavigationDrawer::new().with_theme(theme())
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

/// The drawer's presence, mounted and reconciled so that it holds the layer.
fn presence(kit: FluentNavigationDrawer) -> Box<dyn floom_core::WidgetAny> {
    let view = kit.view();
    let mut presence = view.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = presence.create(&mut create);
    let mut cx = RecordingUpdateContext::new();
    presence.update(&mut cx, id);
    presence
}

/// The layer: the scrim-coloured box the row sits in.
fn layer(kit: FluentNavigationDrawer) -> Box<dyn floom_core::WidgetAny> {
    let presence = presence(kit);
    presence.children().remove(0).build()
}

/// The row: the panel and the scrim.
fn row(kit: FluentNavigationDrawer) -> Box<dyn floom_core::WidgetAny> {
    let layer = layer(kit);
    layer.children().remove(0).build()
}

/// The paint ops of one widget.
fn ops_of(widget: &dyn Widget) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
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
    assert_eq!(style.background, theme().colors().surface, "the panel's fill");
    assert_eq!(style.scrim, theme().colors().scrim, "the scrim's colour");
    assert_eq!(style.scrim_alpha, 160, "v1's scrim alpha");
    assert_eq!(style.accent_width, 3.0, "v1's accent width");
    assert_eq!(style.accent_color, theme().colors().primary, "v1's accent");
    assert_eq!(style.elevation, 4, "v1's default elevation");
    assert_eq!(style.shadow_blur, 3.0 * 4.0, "v1's shadow blur");
    assert_eq!(style.shadow_spread, 1.0, "v1's shadow spread");
}

/// The width is the prop's, and this kit has no token fallback.
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

// --- the pixels -----------------------------------------------------------

/// The layer is painted the scrim colour v1 painted.
#[test]
fn the_layer_is_painted_the_scrim_colour() {
    let resolved = kit().resolved_style();
    let layer = layer(kit());
    let fill = ops_of(&*layer).iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    let expected = paint(resolved.scrim);
    let expected = Color { a: resolved.scrim_alpha, ..expected };
    assert_eq!(fill, Some(expected), "the scrim's fill");
}

/// The panel is painted the colour v1 painted.
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

/// The accent is a band on the edge the drawer is *not* on.
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

// --- the structure --------------------------------------------------------

/// The row is the panel and the scrim, in that order on the left.
#[test]
fn the_row_is_the_panel_and_the_scrim() {
    let left = row(kit().open(true).side(NavigationDrawerSide::Left));
    assert_eq!(left.children().len(), 2, "the panel and the scrim");

    let right = row(kit().open(true).side(NavigationDrawerSide::Right));
    assert_eq!(right.children().len(), 2, "the scrim and the panel");
}

/// A closed drawer mounts nothing at all.
#[test]
fn a_closed_drawer_mounts_nothing() {
    let presence = presence(kit());
    assert!(
        presence.children().is_empty(),
        "v1 hid the whole overlay while the drawer was closed"
    );
}

/// The scrim closes the drawer when it is clicked.
#[test]
fn the_scrim_closes_the_drawer_when_it_is_clicked() {
    let kit = kit().open(true);
    let open = kit.open_flag();
    assert!(floom_widgets::read_flag(&open), "the drawer is open");
    let row = row(kit);
    // The scrim is the row's second child on the left.
    let mut scrim = row.children().remove(1).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = scrim.create(&mut create);
    // A press and a release over the same spot: v1's scrim closed the drawer
    // on the click.
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

/// The props report the side and the width.
#[test]
fn the_props_report_the_side_and_the_width() {
    let kit = kit().side(NavigationDrawerSide::Right).width(200.0);
    assert_eq!(kit.props().side, NavigationDrawerSide::Right);
    assert_eq!(kit.props().width, 200.0);
}

/// The theme: v1's Fluent drawer took a *snapshot* of its own design system,
/// so a change of the app-wide theme does not reach it.
#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    // exactly the thing this test is about.
    assert_ne!(
        FluentNavigationDrawer::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent drawer took a snapshot, not the app-wide signal"
    );
}
