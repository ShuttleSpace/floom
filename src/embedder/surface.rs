//! Surface (drawing target) descriptors and lifecycle events.

use peniko::kurbo::Size;

/// The kind of drawing surface a backend provides to Floem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceKind {
    /// GPU-accelerated surface (wgpu / Skia / Metal / Vulkan / OpenGL ES).
    GpuSurface,
    /// CPU software rasterizer (tiny-skia fallback).
    SoftwareSurface,
    /// Terminal / cell grid surface (TUI backends).
    CellSurface,
    /// An external texture owned by another system (e.g. a video frame).
    ExternalTexture,
    /// No surface; drive events, layout, and command capture without painting.
    None,
}

/// Describes the size, scale, and kind of a rendering surface.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceDescriptor {
    /// What kind of surface this is.
    pub kind: SurfaceKind,
    /// Logical (CSS-pixel) size.
    pub logical_size: Size,
    /// Physical pixel size.
    pub pixel_size: (u32, u32),
    /// Device scale factor (physical / logical).
    pub scale: f64,
    /// Whether the surface is transparent (alpha compositing).
    pub transparent: bool,
}

/// Surface lifecycle events sent from a backend into Floem.
#[derive(Clone, Debug, PartialEq)]
pub enum SurfaceEvent {
    /// A surface was created and is ready to present to.
    Created(SurfaceDescriptor),
    /// The surface was resized.
    Resized(SurfaceDescriptor),
    /// The GPU device was lost; the surface must be recreated before painting.
    Lost,
    /// A previously lost surface was restored.
    Restored(SurfaceDescriptor),
    /// The window is occluded (e.g. minimized / covered); painting may pause.
    Occluded(bool),
    /// The surface was destroyed.
    Destroyed,
}
