//! Platform-neutral geometry, color, display, motion, and ID types for Floem 2.0.
//!
//! This crate must remain free of OS windowing and GPU backend dependencies.

#![forbid(unsafe_code)]

mod brush;
mod color;
mod display;
mod event;
mod geometry;
mod ids;
mod motion;
mod native;
mod overlay;
mod path;

pub use brush::{Brush, Filter, Shadow};
pub use color::Color;
pub use display::{DisplayInfo, ScaleFactor};
pub use event::{
    Event, ImeUpdate, Key, KeyEvent, KeyPhase, Modifiers, NamedKey, PointerButton, PointerEvent,
    PointerKind, PointerPhase, ScrollEvent,
};
pub use geometry::{Constraints, Insets, Point, Rect, Size, Transform};
pub use ids::{ElementId, NativeViewId, PlatformId, ResourceId, SceneNodeId, SurfaceId, WindowId};
pub use motion::{
    AnimationSpec, CompositorProp, Easing, PresenceChannel, PresenceTransition, RepeatMode,
    RepeatSpec, Transition, cubic_bezier_at, damped_spring,
};
pub use native::{InputMode, NativeCapture, NativeCompositionMode, NativeSnapshot, NativeViewKind};
pub use overlay::{OverlayHandle, OverlayKind};
pub use path::{
    ClipRegion, ClipShape, DEFAULT_FLATTEN_TOLERANCE, LineCap, LineJoin, Path, PathSeg, Polyline,
    Stroke, resolve_clip,
};

#[cfg(test)]
mod dependency_boundary_tests {
    /// Compile-time documentation that this crate's Cargo.toml has an empty
    /// `[dependencies]` table for production code. Runtime CI also runs
    /// `scripts/check-architecture-deps.sh`.
    #[test]
    fn crate_has_no_runtime_dependencies_listed() {
        let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
        let deps_section = manifest
            .split("[dependencies]")
            .nth(1)
            .expect("[dependencies] section")
            .split('[')
            .next()
            .unwrap_or("");
        let non_empty = deps_section.lines().any(|line| {
            let t = line.trim();
            !t.is_empty() && !t.starts_with('#')
        });
        assert!(
            !non_empty,
            "floem-types must not gain Cargo dependencies without an architecture decision"
        );
    }
}
