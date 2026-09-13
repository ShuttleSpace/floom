//! Rounded-rect pipeline tests: a `Container` with a corner radius paints a
//! scene rect that carries the radius, so both command-recording and GPU
//! backends can shape the corners instead of silently squaring them off.

use floem_app::WidgetHost;
use floem_layout::LayoutStyle;
use floem_scene::SceneNode;
use floem_types::Color;
use floem_widgets::Container;

fn fixed(width: f64, height: f64) -> LayoutStyle {
    LayoutStyle {
        width: Some(width as f32),
        height: Some(height as f32),
        ..LayoutStyle::default()
    }
}

/// The widget's radius survives the whole host → scene hop.
#[test]
fn container_radius_reaches_the_scene_rect() {
    let mut host = WidgetHost::new();
    host.set_size(floem_core::Size::new(240.0, 160.0));
    host.mount_root(
        fixed(120.0, 48.0),
        Container::new()
            .background(Color::rgb(30, 30, 30))
            .radius(8.0),
    );
    host.frame();

    let mut rounded = 0;
    for (_id, node) in host.scene().iter() {
        if let SceneNode::Rect { radius, fill, .. } = node {
            assert_eq!(fill, &Color::rgb(30, 30, 30));
            assert_eq!(*radius, 8.0, "widget radius must reach the scene node");
            rounded += 1;
        }
    }
    assert_eq!(rounded, 1, "the container painted exactly one rect");
}

/// A plain container (radius 0) still paints an unambiguous right-angle rect.
#[test]
fn plain_container_keeps_zero_radius() {
    let mut host = WidgetHost::new();
    host.set_size(floem_core::Size::new(120.0, 80.0));
    host.mount_root(
        fixed(60.0, 24.0),
        Container::new().background(Color::rgb(9, 9, 9)),
    );
    host.frame();

    for (_id, node) in host.scene().iter() {
        if let SceneNode::Rect { radius, .. } = node {
            assert_eq!(*radius, 0.0);
        }
    }
}
