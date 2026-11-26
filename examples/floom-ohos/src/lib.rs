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
            },
        );
        if let Err(error) = outcome {
            eprintln!("floom-ohos: {error}");
        }
    });
}
