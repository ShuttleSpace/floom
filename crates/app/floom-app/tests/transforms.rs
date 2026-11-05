// SPDX-License-Identifier: MIT
//! A6/B7 — transform-class composites become real group scopes.
//!
//! Scale and rotate cannot fold into a node's payloads (scaling each rect
//! about its own centre tears the picture apart), so the paint walk brackets
//! the animated node's picture in a `GroupPush`/`GroupPop` scope pivoting
//! about the node's own box centre. The observables asserted here:
//!
//! * the bracket is in the scene, in order, carrying the transform the
//!   animation's sampled value says — pivoting about the node's centre;
//! * the drawn picture inside it moves frame to frame as the value advances;
//! * the negative space — cancelling the animation retires the markers, so a
//!   finished scale can never leave a frozen transform behind.

use floom_animation::{AnimKind, Tween};
use floom_app::WidgetHost;
use floom_layout::LayoutStyle;
use floom_scene::SceneNode;
use floom_types::{CompositorProp, Length, Point, Rect, Size, Transform};
use floom_widgets::Canvas;

fn rect_canvas() -> Canvas {
    Canvas::new(|cx| {
        let b = cx.bounds();
        let rect =
            Rect::from_origin_size(Point::new(b.x0 + 10.0, b.y0 + 10.0), Size::new(20.0, 10.0));
        cx.fill_rect(rect, floom_types::Color::rgb(30, 30, 30));
    })
    .size(40.0, 30.0)
}

fn host_with_canvas() -> (WidgetHost, floom_app::NodeId) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 60.0));
    let root = host.mount_root(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(60.0)),
            ..LayoutStyle::default()
        },
        Container::new(),
    );
    let canvas = host.mount(
        root,
        LayoutStyle {
            width: Some(Length::Px(40.0)),
            height: Some(Length::Px(30.0)),
            ..LayoutStyle::default()
        },
        rect_canvas(),
    );
    host.frame();
    for _ in 0..10 {
        if !host.needs_frame() {
            break;
        }
        host.frame();
    }
    assert!(!host.needs_frame(), "the host did not settle");
    (host, canvas)
}

/// The `GroupPush`/`GroupPop` pair in the scene, if any.
fn group_bracket(host: &WidgetHost) -> Option<(Transform, bool)> {
    let mut push = None;
    let mut pop = false;
    for (_, node) in host.scene().iter() {
        match node {
            SceneNode::GroupPush { transform } => push = Some(*transform),
            SceneNode::GroupPop => pop = true,
            _ => {}
        }
    }
    push.map(|t| (t, pop))
}

/// The single rect the canvas paints.
fn canvas_rect(host: &WidgetHost) -> Rect {
    host.scene()
        .iter()
        .find_map(|(_, node)| match node {
            SceneNode::Rect { bounds, .. } => Some(*bounds),
            _ => None,
        })
        .expect("the canvas painted its rect")
}

#[test]
fn a_scale_animation_brackets_the_picture_and_the_bracket_moves() {
    let (mut host, canvas) = host_with_canvas();
    let before = canvas_rect(&host);
    assert!(group_bracket(&host).is_none(), "no scale, no scope yet");

    // 0.5× → 1.5× about the canvas's own centre.
    host.animate(
        canvas,
        AnimKind::Tween(Tween::new(0.5, 1.5, 10.0).with_compositor(CompositorProp::Scale(1.0))),
    );
    host.frame();
    for _ in 0..5 {
        host.frame();
    }

    let (transform, closed) = group_bracket(&host).expect("a scale animation opens a scope");
    assert!(closed, "and the walk closes it after the subtree");

    // The scope pivots about the node's laid-out box centre, whatever the
    // sampled value is.
    let b = host.bounds(canvas);
    let centre = Point::new(b.x0 + b.width() / 2.0, b.y0 + b.height() / 2.0);
    let value = host.animation_value(canvas).expect("the tween is running");
    let expected = Transform::scale_around(value, value, centre);
    assert!(
        (transform.a - expected.a).abs() < 1e-9 && (transform.f - expected.f).abs() < 1e-9,
        "the scope carries the sampled scale about the box centre: {transform:?} vs {expected:?}"
    );

    // The bracket wraps the node's picture: push before the rect, pop after.
    let order: Vec<&str> = host
        .scene()
        .iter()
        .map(|(_, node)| match node {
            SceneNode::GroupPush { .. } => "push",
            SceneNode::GroupPop => "pop",
            SceneNode::Rect { .. } => "rect",
            _ => "other",
        })
        .collect();
    let push_at = order.iter().position(|o| *o == "push").unwrap();
    let rect_at = order.iter().position(|o| *o == "rect").unwrap();
    let pop_at = order.iter().position(|o| *o == "pop").unwrap();
    assert!(
        push_at < rect_at && rect_at < pop_at,
        "the bracket surrounds the picture: {order:?}"
    );

    // The payloads are untouched — the transform lives in the scope, which is
    // the whole point: no per-rect folding.
    let after = canvas_rect(&host);
    assert_eq!(before, after, "the rect's own bounds never move");
}

#[test]
fn a_rotate_animation_turns_the_scope_with_the_value() {
    let (mut host, canvas) = host_with_canvas();
    host.animate(
        canvas,
        AnimKind::Tween(Tween::new(0.0, 2.0, 10.0).with_compositor(CompositorProp::Rotate(1.0))),
    );
    host.frame();
    for _ in 0..5 {
        host.frame();
    }

    let (transform, closed) = group_bracket(&host).expect("a rotate animation opens a scope");
    assert!(closed, "and the walk closes it");

    let b = host.bounds(canvas);
    let centre = Point::new(b.x0 + b.width() / 2.0, b.y0 + b.height() / 2.0);
    let value = host.animation_value(canvas).expect("the tween is running");
    let expected = Transform::rotate_around(value, centre);
    assert!(
        (transform.a - expected.a).abs() < 1e-9 && (transform.b - expected.b).abs() < 1e-9,
        "the scope carries the sampled rotation about the box centre: {transform:?} vs {expected:?}"
    );

    // And it advances: two frames apart, the sampled angle differs, so the
    // scope's matrix differs too.
    let first = transform;
    host.frame();
    let (second, _) = group_bracket(&host).expect("still running");
    assert_ne!(
        first.a, second.a,
        "the sampled angle moved, so the matrix moved"
    );
}

#[test]
fn cancelling_the_scale_retires_the_markers() {
    let (mut host, canvas) = host_with_canvas();
    let id = host.animate(
        canvas,
        AnimKind::Tween(Tween::new(0.5, 1.5, 10.0).with_compositor(CompositorProp::Scale(1.0))),
    );
    host.frame();
    assert!(group_bracket(&host).is_some(), "the scope is open");

    host.cancel_animation(canvas);
    let _ = id;
    host.frame();
    host.frame();
    assert!(
        group_bracket(&host).is_none(),
        "a cancelled scale leaves no frozen scope behind"
    );
}

// The test needs a container to mount under; pull it from the prelude-shaped
// surface the facade re-exports so this file compiles against floom-app only.
use floom_widgets::Container;
