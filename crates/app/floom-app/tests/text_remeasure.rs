// SPDX-License-Identifier: MIT
//! Responsive text re-measurement and per-frame damage tracking.
//!
//! Text leaves without a pinned extent size themselves through a taffy measure
//! callback that reads live widget state, so content edits and container
//! reflows re-measure instead of serving the mount-time extent. Painting
//! reuses scene nodes positionally (`Scene::upsert`), which makes
//! `Scene::damage()` reflect exactly the region that changed this frame.

use floom_app::WidgetHost;
use floom_layout::LayoutStyle;
use floom_scene::SceneNode;
use floom_types::{Rect, Size};
use floom_widgets::{Container, Text};

fn text_bounds(host: &WidgetHost, _node: floom_app::NodeId) -> Rect {
    host.scene()
        .iter()
        .find_map(|(_id, scene_node)| match scene_node {
            SceneNode::Text { origin, size, .. } => Some(Rect::from_origin_size(*origin, *size)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no text node in the scene"))
}

fn node_bounds(host: &WidgetHost, node: floom_app::NodeId) -> Rect {
    host.bounds(node)
}

fn overlaps(a: &Rect, b: &Rect) -> bool {
    a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1
}

#[test]
fn text_reflows_when_the_viewport_shrinks() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(300.0, 200.0));
    // A column root: the text child stretches across the viewport (so the
    // measure fn sees the real width constraint) and keeps an auto height
    // (so wrapping is visible in the extent).
    let root = host.mount_root(
        LayoutStyle {
            flex_direction: floom_layout::FlexDirection::Column,
            ..LayoutStyle::default()
        },
        Container::new(),
    );
    let label = host.mount(
        root,
        LayoutStyle::default(),
        Text::new("the quick brown fox jumps over the lazy dog again and again"),
    );
    host.frame();

    let wide = node_bounds(&host, label);
    assert!(
        (wide.width() - 300.0).abs() < 0.5,
        "stretched across the viewport, got {wide:?}"
    );

    host.set_size(Size::new(80.0, 200.0));
    host.frame();
    let narrow = node_bounds(&host, label);
    assert!(
        narrow.width() <= 80.0,
        "wrapped inside the narrow viewport, got {narrow:?}"
    );
    assert!(
        narrow.height() > wide.height(),
        "wrapping grows the extent: {narrow:?} vs {wide:?}"
    );

    // The painted text node must carry the re-measured extent too.
    let painted = text_bounds(&host, label);
    assert!(painted.height() >= narrow.height() - 0.5);
}

#[test]
fn content_edit_remeasures_on_the_next_frame() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(300.0, 200.0));
    let root = host.mount_root(
        LayoutStyle {
            flex_direction: floom_layout::FlexDirection::Column,
            ..LayoutStyle::default()
        },
        Container::new(),
    );
    let label = host.mount(root, LayoutStyle::default(), Text::new("short"));
    host.frame();
    let before = node_bounds(&host, label);

    host.widget_mut::<Text>(label)
        .expect("text widget")
        .set_content("short but now considerably longer, wrapping twice");
    host.frame();
    let after = node_bounds(&host, label);

    assert!(
        after.height() > before.height(),
        "extent follows the new content: {after:?} vs {before:?}"
    );
}

#[test]
fn unchanged_frames_damage_nothing() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(300.0, 200.0));
    let root = host.mount_root(LayoutStyle::default(), Container::new());
    host.mount(root, LayoutStyle::default(), Text::new("stable"));
    host.frame();
    assert!(!host.scene().damage().is_empty(), "first paint paints");

    host.frame();
    assert!(
        host.scene().damage().is_empty(),
        "identical repaint damages nothing: {:?}",
        host.scene().damage().rects()
    );
}

#[test]
fn content_edit_damages_only_the_text_region() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(300.0, 200.0));
    let root = host.mount_root(LayoutStyle::default(), Container::new());
    let label = host.mount(root, LayoutStyle::default(), Text::new("short"));
    host.frame();
    let before = text_bounds(&host, label);

    host.widget_mut::<Text>(label)
        .expect("text widget")
        .set_content("a considerably longer line of text");
    host.frame();

    let damage = host.scene().damage();
    assert!(
        !damage.is_full_frame(),
        "a text edit is not a full-frame event"
    );
    let rects = damage.rects();
    assert!(!rects.is_empty(), "the edit damaged something");
    // Every damaged rect must overlap the old or new text extent — nothing
    // else on this static tree changed.
    let after = text_bounds(&host, label);
    for rect in rects {
        let touches = overlaps(rect, &before) || overlaps(rect, &after);
        assert!(
            touches,
            "damage rect {rect:?} outside old {before:?} / new {after:?}"
        );
    }
}
