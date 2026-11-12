// SPDX-License-Identifier: MIT
//! The embedded platform, end to end.
//!
//! The point of this file is that the embedded path *draws*. A platform that
//! accepts events and owns a buffer is a data structure; what makes it a
//! platform is that a scene rendered through the software rasteriser lands in
//! the framebuffer, at the right pixel, with the right colour.

use floom_platform::{Platform, PlatformEvent, Surface, Window, WindowConfig};
use floom_platform_embedded::EmbeddedPlatform;
use floom_renderer::{FrameInfo, render_once};
use floom_renderer_software::SoftwareRenderer;
use floom_scene::{Scene, SceneNode};
use floom_types::{Brush, Color, Point, Rect, Size};

const W: u32 = 64;
const H: u32 = 64;

/// A scene with one filled rectangle in it.
fn scene_with_a_rect() -> Scene {
    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::new(16.0, 16.0), Size::new(32.0, 32.0)),
        fill: Brush::from(Color::rgb(220, 40, 40)),
        radius: 0.0,
        shadow: None,
    });
    scene.finish_update();
    scene
}

/// A platform with one window and a framebuffer.
fn platform() -> EmbeddedPlatform {
    let mut platform = EmbeddedPlatform::new();
    platform.create_window(WindowConfig {
        title: "embedded".into(),
        size: Size::new(W as f64, H as f64),
        resizable: true,
    });
    platform
}

/// The surface is in memory: there is no native handle to hand out.
#[test]
fn the_surface_is_in_memory() {
    let platform = platform();
    let window = platform.window().expect("a window");
    let surface_id = window.surface_id().expect("a surface");
    let surface = platform.framebuffer().expect("a framebuffer");
    assert_eq!(surface.id(), surface_id);
    assert!(
        surface.target().is_none(),
        "a framebuffer has no native handle, so a GPU presenter must refuse it"
    );
}

/// A rendered scene lands in the framebuffer, pixel for pixel.
///
/// This is the test that makes the embedded path a platform rather than a
/// buffer: the software rasteriser draws the rect, the frame is copied into
/// the surface, and the surface's pixels are the rect's colour inside its
/// bounds and clear outside them.
#[test]
fn a_rendered_scene_lands_in_the_framebuffer() {
    let mut platform = platform();

    let mut renderer = SoftwareRenderer::new(W, H);
    render_once(
        &mut renderer,
        &FrameInfo::software(W, H, 1),
        &scene_with_a_rect(),
    );

    let surface = platform.framebuffer_mut().expect("a framebuffer");
    assert!(
        surface.present(renderer.pixels()),
        "the frame is the surface's own extent, so it must be accepted"
    );

    let inside = surface.pixel(32, 32).expect("a pixel");
    assert_eq!(
        inside,
        [220, 40, 40, 255],
        "the middle of the rect is the rect's colour"
    );
    let outside = surface.pixel(2, 2).expect("a pixel");
    assert_eq!(
        outside,
        [0, 0, 0, 0],
        "outside the rect the framebuffer is untouched"
    );
}

/// A frame of the wrong size is refused rather than copied in half.
#[test]
fn a_frame_of_the_wrong_size_is_refused() {
    let mut platform = platform();
    let surface = platform.framebuffer_mut().expect("a framebuffer");
    assert!(!surface.present(&[0, 0, 0]), "a short frame is not a frame");
}

/// Events reach the app in the order they were pushed.
#[test]
fn events_are_delivered_in_order() {
    let mut platform = platform();
    platform.pointer_moved(Point::new(3.0, 4.0));
    platform.pointer_moved(Point::new(5.0, 6.0));

    let mut events = Vec::new();
    platform.poll_events(&mut events);
    let positions: Vec<_> = events
        .iter()
        .filter_map(|event| match event {
            PlatformEvent::PointerMoved { position, .. } => Some(*position),
            _ => None,
        })
        .collect();
    assert_eq!(
        positions,
        vec![Point::new(3.0, 4.0), Point::new(5.0, 6.0)],
        "the app sees the pointer where it went, in order"
    );

    // The queue is drained, not replayed.
    let mut again = Vec::new();
    platform.poll_events(&mut again);
    assert!(
        again.is_empty(),
        "a drained queue stays drained: {} left",
        again.len()
    );
}

/// A resize moves the framebuffer with the window.
#[test]
fn a_resize_moves_the_framebuffer() {
    let mut platform = platform();
    platform.resize(Size::new(128.0, 96.0));

    let surface = platform.framebuffer().expect("a framebuffer");
    assert_eq!(surface.extent(), (128, 96));
    assert_eq!(
        surface.pixels().len(),
        128 * 96 * 4,
        "the buffer is the window's size, not the old one"
    );
}

/// The platform claims what it can do, and no more.
///
/// An embedded target has a window and a software rasteriser. It has no
/// clipboard, no IME surface and no accessibility tree for the OS to read, so
/// claiming those would be a lie a host would act on.
#[test]
fn the_platform_claims_only_what_it_can_do() {
    let platform = platform();
    let caps = platform.capabilities();
    assert!(caps.supports(floom_platform::Capability::WINDOWING));
    assert!(caps.supports(floom_platform::Capability::SOFTWARE_RENDER));
    assert!(!caps.supports(floom_platform::Capability::CLIPBOARD));
    assert!(!caps.supports(floom_platform::Capability::IME));
    assert!(!caps.supports(floom_platform::Capability::ACCESSIBILITY));
}

/// A frame request is remembered until the host asks for it.
#[test]
fn a_frame_request_is_remembered() {
    let mut platform = platform();
    let window = platform.window().expect("a window");
    let id = window.id();
    platform.request_frame(id);
    assert!(platform.redraw_requested());
    assert!(platform.take_redraw(), "taking it hands over the request");
    assert!(
        !platform.redraw_requested(),
        "and clears it: a redraw asked for twice is two redraws, not one flag"
    );
}
