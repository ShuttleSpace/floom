//! Presentation seam: getting a finished [`Scene`] onto a real surface.
//!
//! [`Renderer`](crate::Renderer) answers "how do I turn a scene into pixels".
//! This module answers the separate question "where do those pixels go, and who
//! owns the thing they go into". Keeping them apart matters because the two have
//! very different lifetimes: a scene is rebuilt every frame, while a swap-chain
//! is created once per window and reconfigured on resize.
//!
//! ## The lifetime contract
//!
//! [`AttachedSurface::handles`] are *borrowed* OS handles. They are valid from
//! the moment [`Presenter::attach`] is called until the matching
//! [`Presenter::detach`], and the platform that produced them must outlive the
//! presenter. [`crate`] does not enforce that — it cannot, because the handles
//! come from a window it does not own — so the app layer is responsible for
//! dropping the presenter before the platform. [`crate::AttachedSurface`] states
//! the contract once, here, instead of each backend re-deriving it.
//!
//! A GPU backend that needs to build a surface from raw handles therefore has to
//! go through `create_surface_unsafe`-style APIs. That unsafety is *inherent to
//! the problem* (the handle is a raw pointer into another subsystem) and is
//! confined to one place per backend rather than spread through the runner.

use floem_scene::{ResourceRegistry, Scene};
use floem_types::{Size, SurfaceId, WindowId};

use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

/// Native handles for a surface that is ready to be presented into.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceHandles {
    pub window: RawWindowHandle,
    pub display: RawDisplayHandle,
}

/// A surface a presenter has been asked to draw into.
///
/// `size` is logical and `scale` is the device pixel ratio; a backend derives
/// its physical extent with [`AttachedSurface::physical_size`].
#[derive(Debug, Clone, Copy)]
pub struct AttachedSurface {
    pub window: WindowId,
    pub surface: SurfaceId,
    pub size: Size,
    pub scale: f64,
    /// `None` for headless/in-memory surfaces (fake platforms, offscreen tests,
    /// framebuffer targets). A GPU presenter must refuse those; a software
    /// presenter does not care.
    pub handles: Option<SurfaceHandles>,
}

impl AttachedSurface {
    pub fn new(
        window: WindowId,
        surface: SurfaceId,
        size: Size,
        scale: f64,
        handles: Option<SurfaceHandles>,
    ) -> Self {
        Self {
            window,
            surface,
            size,
            scale,
            handles,
        }
    }

    /// Physical pixel extent, rounded up so a fractional logical size never
    /// leaves an unpainted sliver.
    pub fn physical_size(&self) -> (u32, u32) {
        let scale = if self.scale > 0.0 { self.scale } else { 1.0 };
        (
            (self.size.width.max(0.0) * scale).ceil() as u32,
            (self.size.height.max(0.0) * scale).ceil() as u32,
        )
    }
}

/// A thing that can put scenes on a surface.
///
/// Implementations are intentionally *not* required to be `Send`: the handles
/// in [`AttachedSurface`] are only meaningful on the thread that owns the
/// window, so a presenter is a thread-affine object by construction.
pub trait Presenter {
    /// Start presenting into `surface`.
    ///
    /// Returns `false` when this presenter cannot serve that surface — a GPU
    /// presenter handed a headless surface, or a device that refused to
    /// initialise. The caller is expected to fall back (offscreen, software, or
    /// no output) rather than treat it as fatal.
    fn attach(&mut self, surface: AttachedSurface) -> bool;

    /// The surface changed size and/or scale.
    fn resize(&mut self, size: Size, scale: f64);

    /// Draw `scene` and make it visible.
    ///
    /// `resources` is the registry backing the scene: nodes reference glyph
    /// runs and images by id, and resolving those payloads needs the table
    /// that owns them. Backends that cannot use a resource kind (a CPU
    /// renderer without text, say) skip them rather than fail the frame.
    fn present(&mut self, scene: &Scene, resources: &ResourceRegistry);

    /// The surface is going away (window closed, display removed). Release the
    /// swap-chain but keep the device, so a later `attach` is cheap.
    fn detach(&mut self) {}

    /// Frames presented so far, for diagnostics.
    fn frames_presented(&self) -> u64 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floem_types::Point;

    fn window() -> WindowId {
        WindowId::new(0, 1)
    }

    fn surface() -> SurfaceId {
        SurfaceId::new(0, 1)
    }

    #[test]
    fn physical_size_follows_scale() {
        let attached =
            AttachedSurface::new(window(), surface(), Size::new(800.0, 600.0), 2.0, None);
        assert_eq!(attached.physical_size(), (1600, 1200));
    }

    #[test]
    fn fractional_logical_size_rounds_up() {
        let attached =
            AttachedSurface::new(window(), surface(), Size::new(100.5, 50.25), 1.0, None);
        assert_eq!(attached.physical_size(), (101, 51));
    }

    #[test]
    fn zero_and_negative_scale_is_treated_as_one() {
        let attached = AttachedSurface::new(window(), surface(), Size::new(10.0, 10.0), 0.0, None);
        assert_eq!(attached.physical_size(), (10, 10));
    }

    #[test]
    fn headless_surface_has_no_handles() {
        let attached = AttachedSurface::new(window(), surface(), Size::new(4.0, 4.0), 1.0, None);
        assert!(attached.handles.is_none());
        // A presenter that refuses headless surfaces is the normal case; only
        // the software one accepts them.
        assert_eq!(
            Point::new(attached.size.width, attached.size.height),
            Point::new(4.0, 4.0)
        );
    }
}
