//! OpenHarmony (HarmonyOS) embedder backend (experimental skeleton).
//!
//! This mirrors `android.rs` / `ios.rs`. The bulk of event handling reuses
//! Floom's existing winit path, because `floom-winit-ohos` already translates
//! the XComponent surface + touch/key/mouse input into standard winit
//! `WindowEvent`s. The OHOS-specific work here is limited to:
//!
//! - reporting OpenHarmony [`BackendCapabilities`],
//! - a placeholder [`PlatformServices`] (the clipboard / external-URL /
//!   file-dialog / receive-files bridges are drafted as N-API bridges in
//!   `crate::platform::ohos_*`, see the Phase 3 plan and `docs/ohos.md`),
//! - an accessibility backend (`crate::accessibility::backend_ohos`) that
//!   serializes the neutral a11y tree to ArkTS via the shared N-API bridge,
//! - describing an OpenHarmony [`SurfaceDescriptor`] from the native window
//!   size and scale.
//!
//! The actual entry point is wired in `crate::app` via
//! `floom_winit_ohos::export_ohos_winit_app!(floom_app_factory)`; this module
//! only supplies the capability / service / surface descriptions.

#![cfg(target_env = "ohos")]

use super::{BackendCapabilities, EmbedderWindowId, SurfaceDescriptor, SurfaceKind};
use super::{EmbedderWindow, PlatformServices, SurfaceHandle};

/// Capabilities reported by the OpenHarmony backend.
///
/// Mobile-like: no window title bar, no OS-level file drag-and-drop. The
/// N-API platform bridges (clipboard / file_dialog / receive_files /
/// open_external / accessibility) are drafted in `crate::platform::ohos_*`
/// and `crate::accessibility::backend_ohos`; they are wired but **unverified
/// on a real OHOS SDK build** (see `docs/ohos.md`). IME is reported `true`
/// but only enable/disable is supported by `floom-winit-ohos` (deep
/// soft-keyboard integration is a follow-up).
pub fn ohos_capabilities() -> BackendCapabilities {
    BackendCapabilities {
        clipboard: true,      // N-API bridge drafted (crates/floom/src/platform/ohos_clipboard.rs)
        cursor: true,
        timers: true,
        file_dialog: true,    // N-API bridge drafted (crates/floom/src/platform/ohos_file.rs)
        context_menu: true,
        window_title: false,
        // NOTE (#113/#121): claimed true, but the OHOS screen reader is NOT
        // actually reachable yet. The ArkTS AccessibilityProvider design was
        // invalid (no such API in @kit.AccessibilityKit); real exposure needs
        // the native C ArkUI_AccessibilityProvider (see backend_ohos.rs, #121).
        accessibility: true, // claimed; native C provider pending (#121)
        system_theme: true,
        display_info: true,
        safe_area: true,
        virtual_keyboard: true,
        open_external: true, // N-API bridge drafted (crates/floom/src/platform/ohos_external.rs)
        receive_files: true, // N-API bridge drafted (crates/floom/src/platform/ohos_receive.rs)
        drag_drop: false,
        ime: true,            // shallow: enable/disable only
        gpu_surface: true,
        software_surface: false,
        headless: false,
    }
}

/// OpenHarmony platform services.
///
/// Placeholder: the real implementations delegate to an N-API bridge that is
/// not written yet. `capabilities()` already reports the eventual set; the
/// data-path methods return errors until the bridge lands.
pub struct OhosServices {
    // Reserved for the N-API bridge handles.
}

impl OhosServices {
    /// Create the OpenHarmony services backend.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for OhosServices {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformServices for OhosServices {
    fn capabilities(&self) -> BackendCapabilities {
        ohos_capabilities()
    }

    fn read_clipboard(&mut self) -> Result<String, String> {
        crate::platform::Clipboard::get_contents().map_err(|e| e.to_string())
    }

    fn write_clipboard(&mut self, text: String) -> Result<(), String> {
        crate::platform::Clipboard::set_contents(text).map_err(|e| e.to_string())
    }
}

/// OpenHarmony embedder window handle.
///
/// Redraws are driven by `floom-winit-ohos`'s frame pump, so
/// [`EmbedderWindow::request_redraw`] is a no-op here.
pub struct OhosEmbedderWindow {
    id: EmbedderWindowId,
}

impl OhosEmbedderWindow {
    /// Create a window handle for the given protocol window id.
    pub fn new(id: EmbedderWindowId) -> Self {
        Self { id }
    }
}

impl EmbedderWindow for OhosEmbedderWindow {
    fn id(&self) -> EmbedderWindowId {
        self.id
    }

    fn request_redraw(&mut self) {
        // floom-winit-ohos owns the frame pump; nothing to do here.
    }

    fn set_title(&mut self, _title: &str) {
        // OpenHarmony apps are full-screen; ignore.
    }
}

/// OpenHarmony rendering surface descriptor holder.
pub struct OhosSurfaceHandle {
    descriptor: SurfaceDescriptor,
}

impl OhosSurfaceHandle {
    /// Create a surface handle wrapping `descriptor`.
    pub fn new(descriptor: SurfaceDescriptor) -> Self {
        Self { descriptor }
    }
}

impl SurfaceHandle for OhosSurfaceHandle {
    fn descriptor(&self) -> SurfaceDescriptor {
        self.descriptor.clone()
    }
}

/// Build a [`SurfaceDescriptor`] for an OpenHarmony GPU surface.
///
/// `pixel_size` is the native window's pixel dimensions and `scale` the device
/// scale factor. OpenHarmony uses an opaque (`transparent = false`) GPU
/// surface. Floom's renderer (`wgpu::Instance::create_surface`, via
/// `VK_OHOS_surface` on wgpu >= 0.30) consumes the native window behind this
/// descriptor.
pub fn ohos_surface_descriptor(
    logical_size: peniko::kurbo::Size,
    pixel_size: (u32, u32),
    scale: f64,
) -> SurfaceDescriptor {
    SurfaceDescriptor {
        kind: SurfaceKind::GpuSurface,
        logical_size,
        pixel_size,
        scale,
        transparent: false,
    }
}
