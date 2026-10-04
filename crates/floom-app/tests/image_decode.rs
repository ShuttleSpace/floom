//! Image decode pipeline tests: encoded PNG bytes → `decode_image`
//! (premultiplied RGBA8) → `WidgetHost::register_image_decoded` → a scene
//! node an `Image` widget renders from.

use floom_core::Length;
use floom_app::{WidgetHost, decode_image};
use floom_scene::{ResourceKind, SceneNode};
use std::io::Cursor;

/// Encodes a 3×2 RGBA image to PNG bytes, with two transparent pixels so the
/// premultiply step has something to prove.
fn sample_png() -> Vec<u8> {
    let mut img = image::RgbaImage::new(3, 2);
    // Row 0: opaque red, opaque green, transparent black.
    img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
    img.put_pixel(1, 0, image::Rgba([0, 255, 0, 255]));
    img.put_pixel(2, 0, image::Rgba([0, 0, 0, 0]));
    // Row 1: half-alpha blue, half-alpha white, opaque gray.
    img.put_pixel(0, 1, image::Rgba([0, 0, 255, 128]));
    img.put_pixel(1, 1, image::Rgba([255, 255, 255, 128]));
    img.put_pixel(2, 1, image::Rgba([40, 40, 40, 255]));
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, image::ImageFormat::Png)
        .expect("PNG encode");
    buf.into_inner()
}

#[test]
fn decodes_png_to_premultiplied_rgba8() {
    let decoded = decode_image(&sample_png()).expect("valid PNG");
    assert_eq!((decoded.width, decoded.height), (3, 2));
    assert_eq!(decoded.pixels.len(), 3 * 2 * 4);

    let px = |i: usize| &decoded.pixels[i * 4..i * 4 + 4];
    // Opaque pixels pass through untouched.
    assert_eq!(px(0), &[255, 0, 0, 255]);
    assert_eq!(px(1), &[0, 255, 0, 255]);
    // Transparent pixel: premultiplied to black.
    assert_eq!(px(2), &[0, 0, 0, 0]);
    // Half-alpha blue: 255*128/255 → 128.
    assert_eq!(px(3), &[0, 0, 128, 128]);
    // Half-alpha white: all channels scale to 128.
    assert_eq!(px(4), &[128, 128, 128, 128]);
}

#[test]
fn garbage_bytes_are_rejected() {
    assert!(decode_image(b"not an image at all").is_none());
    assert!(decode_image(&[]).is_none());
    // A PNG signature followed by junk must not panic.
    let mut truncated = sample_png();
    truncated.truncate(20);
    assert!(decode_image(&truncated).is_none());
}

#[test]
fn register_image_decoded_roundtrips_through_the_registry() {
    let mut host = WidgetHost::new();
    let bytes = sample_png();
    let id = host
        .register_image_decoded(&bytes)
        .expect("valid PNG registers");

    match host.resources().get(id).map(|r| &r.kind) {
        Some(ResourceKind::Image {
            width,
            height,
            pixels,
        }) => {
            assert_eq!((*width, *height), (3, 2));
            assert_eq!(pixels.len(), 24);
            // Premultiplied, not straight alpha: half-alpha blue is 128, not 255.
            assert_eq!(pixels[12..16], [0, 0, 128, 128]);
        }
        other => panic!("expected an image resource, got {other:?}"),
    }
}

#[test]
fn failed_decode_leaves_the_registry_untouched() {
    let mut host = WidgetHost::new();
    assert!(host.register_image_decoded(b"garbage").is_none());
    assert_eq!(host.resources().used_units(), 0, "nothing was interned");
}

#[test]
fn decoded_image_paints_a_scene_node() {
    let mut host = WidgetHost::new();
    host.set_size(floom_core::Size::new(120.0, 80.0));
    let id = host
        .register_image_decoded(&sample_png())
        .expect("valid PNG registers");
    let root = host.mount_root(
        floom_layout::LayoutStyle::default(),
        floom_widgets::Container::new(),
    );
    host.mount(
        root,
        floom_layout::LayoutStyle {
            width: Some(Length::Px(24.0)),
            height: Some(Length::Px(16.0)),
            ..floom_layout::LayoutStyle::default()
        },
        floom_widgets::Image::new(id).intrinsic_size(floom_core::Size::new(3.0, 2.0)),
    );
    host.frame();

    let images: Vec<_> = host
        .scene()
        .iter()
        .filter_map(|(_id, node)| match node {
            SceneNode::Image { image, .. } => Some(*image),
            _ => None,
        })
        .collect();
    assert_eq!(images, vec![id], "the decoded resource is referenced");
}
