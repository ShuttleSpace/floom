//! A3/A4 through the real host: what a widget asks for is what the retained
//! scene carries.
//!
//! The renderers prove the pixels (`floom-renderer-software/tests/brush.rs`);
//! this file proves the *plumbing a widget actually uses* — `Container`'s
//! gradient background and elevation, and a `Canvas` painter drawing with a
//! brush — survives the whole way into scene nodes, in the order the draw
//! call made them.

use floom_app::WidgetHost;
use floom_layout::LayoutStyle;
use floom_scene::SceneNode;
use floom_types::{Brush, Color, Path, Point, Rect, Shadow, Size};
use floom_widgets::{Canvas, Container};

const RAMP: fn() -> Brush = || {
    Brush::linear(
        &[
            (0.0, Color::rgb(30, 60, 220)),
            (1.0, Color::rgb(220, 60, 30)),
        ],
        Point::new(20.0, 20.0),
        Point::new(140.0, 20.0),
    )
};

fn box_style(width: f32, height: f32) -> LayoutStyle {
    LayoutStyle {
        width: Some(width),
        height: Some(height),
        ..LayoutStyle::default()
    }
}

/// Mounts `widget` as the root of a 200×200 host and settles the frame loop.
fn host_with(widget: impl floom_core::WidgetAny + 'static) -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 200.0));
    host.mount_root(box_style(200.0, 200.0), widget);
    host.frame();
    for _ in 0..10 {
        if !host.needs_frame() {
            break;
        }
        host.frame();
    }
    host
}

/// Every rect node the scene holds, in draw order.
fn rects(host: &WidgetHost) -> Vec<(Rect, Brush, Option<Shadow>)> {
    host.scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::Rect {
                bounds,
                fill,
                shadow,
                ..
            } => Some((*bounds, fill.clone(), *shadow)),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// A3 — a gradient background through the widget door
// ---------------------------------------------------------------------------

/// The ramp a container was built with is the ramp the renderer receives —
/// not a colour sampled somewhere on the way down.
#[test]
fn a_gradient_background_reaches_the_scene_intact() {
    let host = host_with(Container::new().background(RAMP()));
    let nodes = rects(&host);
    assert_eq!(nodes.len(), 1, "one card, one node");
    assert_eq!(nodes[0].1, RAMP(), "the whole ramp survived the walk");
    assert!(
        nodes[0].1.solid().is_none(),
        "and it is still a gradient, not one collapsed colour"
    );
    assert!(nodes[0].2.is_none(), "no shadow was asked for");
}

/// A plain colour still paints a plain node: the day before gradients existed
/// nothing about this background changed, including how many nodes it costs.
#[test]
fn a_flat_background_still_costs_one_flat_node() {
    let host = host_with(Container::new().background(Color::rgb(10, 20, 30)));
    let nodes = rects(&host);
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].1.solid(), Some(Color::rgb(10, 20, 30)));
}

// ---------------------------------------------------------------------------
// A4 — elevation through the widget door
// ---------------------------------------------------------------------------

/// A shadow is one extra node in front of the fill it belongs to, carrying the
/// whole request — including a transparent silhouette, because the host cannot
/// know how much ink any given backend will be able to put behind it.
#[test]
fn an_elevated_card_casts_its_shadow_before_its_own_fill() {
    let shadow = Shadow {
        color: Color::rgba(0, 0, 0, 96),
        blur: 6.0,
        offset_x: 0.0,
        offset_y: 4.0,
    };
    let host = host_with(
        Container::new()
            .background(Color::WHITE)
            .radius(8.0)
            .shadow(shadow),
    );
    let nodes = rects(&host);
    assert_eq!(nodes.len(), 2, "a shadow node and the card's own node");
    assert_eq!(
        nodes[0].2,
        Some(shadow),
        "the first node is the cast shadow, request intact"
    );
    assert_eq!(
        nodes[0].1.solid(),
        Some(Color::TRANSPARENT),
        "the shadow node carries no fill of its own to hide the shadow under"
    );
    assert_eq!(nodes[1].2, None, "the card itself casts nothing further");
    assert_eq!(
        nodes[1].1.solid(),
        Some(Color::WHITE),
        "and the card is still white"
    );
    // Both describe the same box, so a shadow scaled or offset by the walker
    // would show up here rather than as a silently drifting halo.
    assert_eq!(nodes[0].0, nodes[1].0);
}

// ---------------------------------------------------------------------------
// A3 — the same door from a self-drawn painter
// ---------------------------------------------------------------------------

/// A `Canvas` painter can paint with a brush too, because the brush ops live on
/// `PaintContext`: the path it fills carries the ramp rather than one colour
/// the painter had to pick in advance.
#[test]
fn a_canvas_painter_can_fill_with_a_brush() {
    let ramp = RAMP();
    let host = host_with(
        Canvas::new(move |cx| {
            let b = cx.bounds();
            let path = Path::rect(Rect {
                x0: b.x0 + 4.0,
                y0: b.y0 + 4.0,
                x1: b.x1 - 4.0,
                y1: b.y1 - 4.0,
            });
            cx.fill_path_brush(&path, &ramp);
        })
        .size(120.0, 40.0),
    );
    let paths: Vec<_> = host
        .scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::Path { fill, .. } => Some(fill.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(paths, vec![Some(RAMP())], "the path carries the ramp");
}
