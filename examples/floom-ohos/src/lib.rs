// SPDX-License-Identifier: MIT
//! The OpenHarmony demo library: a real Floom tree instead of a gradient.
//!
//! ArkTS owns the XComponent and calls `floom_demo_start()` once the surface is
//! bound, because `floom_harmony_present_frame` refuses frames submitted before
//! `floom_harmony_bind_surface`. The frame loop then lives in Rust: `floom::run`
//! with the harmony backend mounts the tree and presents every frame through the
//! buffer queue this module was linked against.

// Re-exported so the C++ seam can resolve them from this library alone.
pub use floom_platform_harmony::{
    floom_harmony_bind_surface, floom_harmony_present_frame, floom_harmony_release_surface,
    floom_harmony_set_display_scale,
};

use floom_types::Size;
use floom_widgets::{Button, Stack};

/// Start the demo on its own thread.
///
/// A separate thread because `floom::run` is a blocking frame loop and the caller
/// is the ArkTS UI thread: blocking it would freeze the XComponent that owns the
/// surface this loop draws into.
#[unsafe(no_mangle)]
pub extern "C" fn floom_demo_start(width_px: i32, height_px: i32, density: f32) {
    // The frame has to be the surface's pixel size, not the logical one.
    floom_harmony_set_display_scale(density);
    std::thread::spawn(move || {
        let outcome = floom::run(
            "Floom",
            Size::new(
                width_px as f64 / density as f64,
                height_px as f64 / density as f64,
            ),
            |host| {
                let column = host.mount_root(
                    floom_layout::LayoutStyle {
                        flex_direction: floom_layout::FlexDirection::Column,
                        ..floom_layout::LayoutStyle::default()
                    },
                    floom_widgets::Container::new(),
                );
                host.mount(
                    column,
                    floom_layout::LayoutStyle::default(),
                    Stack::new(floom_core::Axis::Vertical).child(floom_core::Child::new(
                        floom_core::ChildKey::index(0),
                        floom_core::LayoutRequest::new(),
                        || Box::new(Button::new("Floom on OpenHarmony")),
                    )),
                );
                // 量一次就够：布局尺寸、缩放、内容盒子，与 present 那行
                // `frame WxH` 对起来，就能看出 scale 在哪一段被多用了一次。
                host.on_frame(|host| {
                    use std::sync::atomic::{AtomicU32, Ordering};
                    static FRAMES: AtomicU32 = AtomicU32::new(0);
                    // The first frame's callback runs before layout, so the
                    // boxes are still zero there; the third is past it.
                    if FRAMES.fetch_add(1, Ordering::Relaxed) != 2 {
                        return;
                    }
                    let ink =
                        host.semantics()
                            .iter()
                            .map(|(_, node)| node.bounds)
                            .reduce(|a, b| floom_types::Rect {
                                x0: a.x0.min(b.x0),
                                y0: a.y0.min(b.y0),
                                x1: a.x1.max(b.x1),
                                y1: a.y1.max(b.y1),
                            });
                    let nodes: Vec<String> = host
                        .tree_nodes()
                        .iter()
                        .take(4)
                        .map(|node| format!("{:?}@{:?}", node.kind, node.bounds))
                        .collect();
                    floom_platform_harmony::log(&format!(
                        "host size={:?} scale={} ink={:?} nodes={} {:?}",
                        host.size(),
                        host.scale(),
                        ink,
                        host.tree_nodes().len(),
                        nodes
                    ));
                });
            },
        );
        if let Err(error) = outcome {
            eprintln!("floom-ohos: {error}");
        }
    });
}
