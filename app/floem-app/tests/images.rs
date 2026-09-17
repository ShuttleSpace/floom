//! Image pipeline tests: `WidgetHost::register_image` validates pixel extents,
//! and the `Image` widget paints a scene node that references the resource —
//! the resource-side contract the GPU presenter resolves through
//! `WidgetHost::resources()`.

use floem_app::WidgetHost;
use floem_layout::LayoutStyle;
use floem_scene::{ResourceKind, SceneNode};
use floem_widgets::{Container, Image};

fn fixed(width: f64, height: f64) -> LayoutStyle {
    LayoutStyle {
        width: Some(width as f32),
        height: Some(height as f32),
        ..LayoutStyle::default()
    }
}

#[test]
fn register_image_validates_pixel_extent() {
    let mut host = WidgetHost::new();
    // Wrong lengths must be refused without touching the registry.
    assert!(host.register_image(2, 2, vec![0; 15]).is_none());
    assert!(host.register_image(2, 2, vec![0; 17]).is_none());
    assert_eq!(host.resources().used_units(), 0, "nothing was interned");

    let id = host
        .register_image(2, 2, vec![128; 16])
        .expect("valid extent");
    match host.resources().get(id).map(|r| &r.kind) {
        Some(ResourceKind::Image {
            width,
            height,
            pixels,
        }) => {
            assert_eq!((*width, *height), (2, 2));
            assert_eq!(pixels.len(), 16);
        }
        other => panic!("expected an image resource, got {other:?}"),
    }
}

#[test]
fn image_widget_paints_a_node_referencing_its_resource() {
    let mut host = WidgetHost::new();
    host.set_size(floem_core::Size::new(240.0, 160.0));
    let img = host
        .register_image(8, 4, vec![255; 8 * 4 * 4])
        .expect("valid extent");
    let root = host.mount_root(LayoutStyle::default(), Container::new());
    host.mount(
        root,
        fixed(64.0, 32.0),
        Image::new(img).intrinsic_size(floem_core::Size::new(8.0, 4.0)),
    );
    host.frame();

    let mut images = 0;
    for (_id, node) in host.scene().iter() {
        if let SceneNode::Image {
            bounds,
            image,
            alpha: _,
        } = node
        {
            images += 1;
            assert_eq!(*image, img, "node references the registered resource");
            assert!(
                bounds.width() > 0.0 && bounds.height() > 0.0,
                "painted image has real bounds, got {bounds:?}"
            );
        }
    }
    assert_eq!(images, 1, "the image widget painted exactly one node");
}
