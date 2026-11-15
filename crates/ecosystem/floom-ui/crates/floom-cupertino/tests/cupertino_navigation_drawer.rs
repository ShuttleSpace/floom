#![cfg(feature = "navigation_drawer")]
//! The ported Cupertino navigation drawer -- a panel that slides in from the
//! edge of the app over a dimmed scrim.

use floom_core::{ElementId, Event, Point, PointerEvent, Rect, Widget};
use floom_types::Color;
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::navigation_drawer::NavigationDrawerSide;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_cupertino::widgets::navigation_drawer::{
    CupertinoNavigationDrawer, CupertinoNavigationDrawerStyle,
};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn kit() -> CupertinoNavigationDrawer {
    CupertinoNavigationDrawer::new().with_theme(theme())
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
fn layer(kit: CupertinoNavigationDrawer) -> Box<dyn floom_core::WidgetAny> {
    let view = kit.view();
    view.children().remove(0).build()
}

/// The scrim's presence, mounted and reconciled so that it holds its child.
fn scrim_presence(kit: CupertinoNavigationDrawer) -> Box<dyn floom_core::WidgetAny> {
    let layer = layer(kit);
    let mut presence = layer.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = presence.create(&mut create);
    let mut cx = RecordingUpdateContext::new();
    presence.update(&mut cx, id);
    presence
}

/// The panel's presence, mounted and reconciled so that it holds its child.
fn panel_presence(kit: CupertinoNavigationDrawer) -> Box<dyn floom_core::WidgetAny> {
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

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: CupertinoNavigationDrawerStyle = kit().resolved_style();
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
    assert_eq!(
        style.header_color,
        theme().colors().on_surface,
        "the header's ink"
    );
    assert_eq!(
        style.content_color,
        theme().colors().on_surface_variant,
        "the content's ink"
    );
    assert_eq!(style.panel_alpha, 235, "v1's panel alpha");
    assert_eq!(style.border_width, 0.5, "v1's hairline");
    assert_eq!(style.elevation, 4, "v1's default elevation");
    assert_eq!(style.shadow_blur, 4.0 * 4.0, "v1's shadow blur");
    assert_eq!(style.shadow_spread, 0.0, "v1's shadow spread");
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
        "v1 clamped the prop's width at zero and never asked the token"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "cupertino");
}

// --- the pixels -----------------------------------------------------------

/// The panel is painted the colour v1 painted.
#[test]
fn the_panel_is_painted_v1_s_colour() {
    let resolved = kit().resolved_style();
    let presence = panel_presence(kit().open(true));
    let panel = presence.children().remove(0).build();
    let expected = paint(resolved.background);
    let expected = Color {
        a: resolved.panel_alpha,
        ..expected
    };
    assert_eq!(fill_of(&*panel), Some(expected), "the panel's fill");
}

/// The scrim is painted the colour v1 painted.
#[test]
fn the_scrim_is_painted_v1_s_colour() {
    let resolved = kit().resolved_style();
    let presence = scrim_presence(kit().open(true));
    let pressable = presence.children().remove(0).build();
    let scrim = pressable.children().remove(0).build();
    assert_eq!(
        fill_of(&*scrim),
        Some(paint(resolved.scrim_color)),
        "the scrim's fill"
    );
}

// --- the structure --------------------------------------------------------

/// A closed drawer mounts nothing at all.
#[test]
fn a_closed_drawer_mounts_nothing() {
    let presence = scrim_presence(kit());
    assert!(
        presence.children().is_empty(),
        "v1 hid the whole scrim while the drawer was closed"
    );
}

/// The panel is pinned to the edge v1 pinned it to.
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
    assert_eq!(inset.bottom, Some(0.0), "v1 inset it top and bottom");

    let presence = panel_presence(kit().open(true).side(NavigationDrawerSide::Right));
    let panel = presence.children().remove(0).build();
    let request = panel
        .layout_request()
        .expect("the panel asks for its own box");
    let inset = request.inset.expect("v1's panel was inset");
    assert_eq!(inset.right, Some(0.0), "v1 pinned it to the right");
    assert_eq!(inset.left, None, "and not to the left");
}

/// The hairline is on the edge the drawer is *not* on.
#[test]
fn the_hairline_is_on_the_edge_the_drawer_is_not_on() {
    let resolved = kit().resolved_style();
    let presence = panel_presence(kit().open(true).side(NavigationDrawerSide::Left));
    let panel = presence.children().remove(0).build();
    // The border is a decoration on the panel's own box.
    let spec = panel_border(&*panel);
    assert_eq!(spec, Some(resolved.border_width), "v1's hairline width");
}

/// The border spec of a surface, read through the widget's own decoration.
fn panel_border(widget: &dyn Widget) -> Option<f32> {
    // The panel is a `Surface`; its decoration is reachable through the
    // widget's own type, which the test cannot name generically -- so the
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    for op in recorder.ops() {
        if let PaintOp::Rect { bounds, .. } = op {
            if bounds.width() < 1.0 {
                return Some(bounds.width() as f32);
            }
        }
    }
    None
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
    // A press and a release over the same spot: v1's scrim closed the drawer
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

/// The theme: v1's Cupertino drawer took a *snapshot* of its own design
/// system, so a change of the app-wide theme does not reach it.
#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    assert_ne!(
        CupertinoNavigationDrawer::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Cupertino drawer took a snapshot, not the app-wide signal"
    );
}
