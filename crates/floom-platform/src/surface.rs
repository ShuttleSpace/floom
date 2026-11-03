//! Surface / render-target contract.
//!
//! Two things live here:
//!
//! * the *logical* surface identity ([`SurfaceId`], logical [`Size`]) that the
//!   widget/runtime layers can talk about without knowing about GPU APIs, and
//! * [`SurfaceTarget`], the *native* handle pair a renderer needs in order to
//!   actually create a swap-chain for that surface.
//!
//! The handle pair is expressed with `raw-window-handle` types. That crate is
//! OS-neutral by construction (it is a set of opaque handle descriptors, not a
//! windowing or GPU library), which is why it is allowed in the ABI crate while
//! `winit` / `wgpu` / `vello` are not. Backends fill [`Surface::target`] in;
//! renderer backends consume it in the render layer.

use floom_types::{Size, SurfaceId};

use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceConfig {
    pub size: Size,
    pub transparent: bool,
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self {
            size: Size::new(800.0, 600.0),
            transparent: false,
        }
    }
}

/// Everything a GPU backend needs to create and configure a swap-chain.
///
/// `size` is in **logical** pixels (consistent with the rest of the ABI) and
/// `scale` is the device pixel ratio, so a renderer can derive the physical
/// extent with [`physical_size`](Self::physical_size) without asking the
/// platform again. The handles borrow from the live window and stay valid only
/// while the source window does, which is why the render layer attaches from
/// the same event that created the surface.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceTarget {
    pub surface: SurfaceId,
    pub size: Size,
    pub scale: f64,
    pub window_handle: RawWindowHandle,
    pub display_handle: RawDisplayHandle,
}

impl SurfaceTarget {
    pub fn new(
        surface: SurfaceId,
        size: Size,
        scale: f64,
        window_handle: RawWindowHandle,
        display_handle: RawDisplayHandle,
    ) -> Self {
        Self {
            surface,
            size,
            scale,
            window_handle,
            display_handle,
        }
    }

    /// Physical size as a `(width, height)` pair, rounded up so a fractional
    /// logical size never leaves an unpainted sliver.
    pub fn physical_size(&self) -> (u32, u32) {
        let scale = if self.scale > 0.0 { self.scale } else { 1.0 };
        (
            (self.size.width.max(0.0) * scale).ceil() as u32,
            (self.size.height.max(0.0) * scale).ceil() as u32,
        )
    }
}

pub trait Surface {
    fn id(&self) -> SurfaceId;
    fn size(&self) -> Size;
    fn resize(&mut self, size: Size);

    /// Native handles for this surface, or `None` for in-memory / headless
    /// surfaces (fake platform, embedded framebuffer, offscreen test doubles).
    fn target(&self) -> Option<SurfaceTarget> {
        None
    }
}
