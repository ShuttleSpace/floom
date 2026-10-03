//! Android embedder backend (experimental skeleton).
//!
//! This is a first cut at implementing the embedder adapter traits for Android.
//! It is intentionally small because the lapce winit fork **already** translates
//! Android input into standard winit `WindowEvent`s — so the bulk of event
//! handling reuses Floom's existing winit path. The Android-specific work here
//! is limited to:
//!
//! - reporting Android [`BackendCapabilities`] (no muda native menu bar, no
//!   window title; in-UI context menu, IME / soft keyboard, safe-area insets,
//!   a GPU surface, the system clipboard, external-URL opening, and the Storage
//!   Access Framework file dialogs are all supported),
//! - bridging the clipboard, external-URL opening, and file dialogs through the
//!   shared JNI handle stored by `Application::new_android`,
//! - describing an Android [`SurfaceDescriptor`] from the native window size
//!   and scale.
//!
//! The remaining hard step is renderer-side surface creation from the Android
//! `ANativeWindow*` (`ndk`). That integration lives alongside the GPU backend
//! and is tracked in `docs/android.md`.

#![cfg(target_os = "android")]

use super::{BackendCapabilities, EmbedderWindowId, SurfaceDescriptor, SurfaceKind};
use super::{EmbedderWindow, PlatformServices, SurfaceHandle};

/// Capabilities reported by the Android backend.
///
/// Compared with [`BackendCapabilities::winit_default`], Android disables the
/// features that have no Android equivalent:
///
/// - `window_title` — Android apps are full-screen; there is no window title bar.
/// - `drag_drop` — the OS-level *file* drag-and-drop surfaced by
///   [`EmbedderEvent::DragDrop`] / the `FileDragDrop` listener. The winit-android
///   backend emits **no** `WindowEvent::DragDropped`/`DragEntered`/`DragMoved`/
///   `DragLeft` events (it only supports `drag_window` for moving the OS window),
///   and Android's per-app sandbox has no concept of dragging a file from another
///   app or the file manager *into* a window. So `drag_drop` stays `false` and must
///   not be flipped on: doing so would advertise a capability nothing delivers.
///   The Android idiom for receiving a file from another app is an `Intent`
///   (`ACTION_SEND` / `ACTION_VIEW` with a `content://` URI) or the Storage Access
///   Framework picker — both are handled separately (external-URL + `file_dialog`
///   capabilities), not via `DragDropEvent`.
///
/// Note that this only concerns *OS file* drag-and-drop. Floom's **in-app** drag
/// system (`Event::Drag` / `DragTracker`, driven by [`PointerEvent`]s and Floom's
/// internal pointer capture) is fully platform-agnostic and **already works on
/// Android** through the touch→`PointerEvent` pipeline — no backend code required.
///
/// The clipboard, external-URL opening, file dialogs, and **receiving shared
/// files** (the `receive_files` capability — Android `ACTION_SEND` / `ACTION_VIEW`
/// intents delivered as `content://` URIs) are all wired through the shared JNI
/// bridge (see `crate::platform::android_jni` and `Application::new_android`,
/// plus `crate::platform::set_shared_files_handler`). Everything else that Floom
/// needs on mobile is enabled.
pub fn android_capabilities() -> BackendCapabilities {
    BackendCapabilities {
        clipboard: true,
        cursor: true,
        timers: true,
        file_dialog: true,
        context_menu: true,
        window_title: false,
        accessibility: true,
        system_theme: true,
        display_info: true,
        safe_area: true,
        virtual_keyboard: true,
        open_external: true,
        receive_files: true,
        drag_drop: false,
        ime: true,
        gpu_surface: true,
        software_surface: false,
        headless: false,
    }
}

/// Android platform services (clipboard, theme, external URLs, file dialogs, …).
///
/// Clipboard read/write delegate to the JNI-backed
/// [`crate::platform::clipboard::Clipboard`] (initialized by
/// `Application::new_android` via the shared JNI handle in
/// `crate::platform::android_jni`); external-URL opening and file dialogs use
/// the same bridge. See `docs/android.md`.
pub struct AndroidServices {
    // Reserved for future JNI handles (e.g. content-provider bridges).
}

impl AndroidServices {
    /// Create the Android services backend.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for AndroidServices {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformServices for AndroidServices {
    fn capabilities(&self) -> BackendCapabilities {
        android_capabilities()
    }

    fn read_clipboard(&mut self) -> Result<String, String> {
        crate::platform::clipboard::Clipboard::get_contents()
            .map_err(|e| format!("{e:?}"))
    }

    fn write_clipboard(&mut self, text: String) -> Result<(), String> {
        crate::platform::clipboard::Clipboard::set_contents(text)
            .map_err(|e| format!("{e:?}"))
    }
}

/// Android embedder window handle.
///
/// Redraws are driven by winit's own redraw-request mechanism on Android, so
/// [`EmbedderWindow::request_redraw`] is a no-op here; the window id is tracked
/// so backends can correlate protocol events.
pub struct AndroidEmbedderWindow {
    id: EmbedderWindowId,
}

impl AndroidEmbedderWindow {
    /// Create a window handle for the given protocol window id.
    pub fn new(id: EmbedderWindowId) -> Self {
        Self { id }
    }
}

impl EmbedderWindow for AndroidEmbedderWindow {
    fn id(&self) -> EmbedderWindowId {
        self.id
    }

    fn request_redraw(&mut self) {
        // On Android, winit owns the redraw cycle; nothing to do here.
    }

    fn set_title(&mut self, _title: &str) {
        // Android has no window title bar; ignore.
    }
}

/// Android rendering surface descriptor holder.
///
/// Constructed from the native window geometry by the integration layer that
/// owns the `ANativeWindow*`.
pub struct AndroidSurfaceHandle {
    descriptor: SurfaceDescriptor,
}

impl AndroidSurfaceHandle {
    /// Create a surface handle wrapping `descriptor`.
    pub fn new(descriptor: SurfaceDescriptor) -> Self {
        Self { descriptor }
    }
}

impl SurfaceHandle for AndroidSurfaceHandle {
    fn descriptor(&self) -> SurfaceDescriptor {
        self.descriptor.clone()
    }
}

/// Build a [`SurfaceDescriptor`] for an Android GPU surface.
///
/// `pixel_size` is the native window's pixel dimensions and `scale` the device
/// scale factor. Android uses an opaque (`transparent = false`) GPU surface.
pub fn android_surface_descriptor(
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
