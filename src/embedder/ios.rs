//! iOS embedder backend (experimental skeleton).
//!
//! First cut at implementing the embedder adapter traits for iOS. Like Android,
//! the lapce winit fork already translates iOS touch/keyboard into standard
//! winit `WindowEvent`s — so event handling reuses Floem's existing winit path
//! with no custom input translation. The iOS-specific work is:
//!
//! - reporting iOS [`BackendCapabilities`] (no muda native menu bar, no window
//!   title; in-UI context menu, a Metal GPU surface, display info, and the
//!   clipboard (via `UIPasteboard`, Step 4) are supported; the soft keyboard /
//!   IME (via a hidden `UITextField` first-responder, Step 5) is also wired;
//!   external-URL / file dialogs / shared files / safe-area are wired in later
//!   steps — see `docs/ios.md` §7),
//! - (later) bridging the clipboard, external-URL opening, file dialogs, and
//!   shared files through `objc2`/`objc2-ui-kit` (no JNI on iOS).
//!
//! Because this is a skeleton, the capabilities below reflect what is *wired*
//! so far; items not yet implemented stay `false` and flip to `true` as the
//! corresponding bridge lands (mirroring how `android.rs` grew over time).

#![cfg(target_os = "ios")]

use super::{BackendCapabilities, EmbedderWindowId, SurfaceDescriptor, SurfaceKind};
use super::{EmbedderWindow, PlatformServices, SurfaceHandle};
use objc2_foundation::MainThreadMarker;
use objc2_ui_kit::{UIApplication, UIEdgeInsets, UIScreen};
use winit::dpi::PhysicalInsets;

/// Capabilities reported by the iOS backend (Step 1 skeleton).
///
/// Compared with [`BackendCapabilities::winit_default`], iOS disables the
/// features that have no iOS equivalent or are not yet implemented:
///
/// - `window_title` — iOS apps are full-screen; there is no window title bar.
/// - `clipboard` — **implemented (Step 4)**: read/write via `UIPasteboard`
///   (`platform/ios_clipboard.rs`), bypassing `copypasta` on iOS (the same
///   path the core `Clipboard` API uses).
/// - `open_external` / `file_dialog` / `receive_files` — backed by
///   `UIApplication.open` / `UIDocumentPickerViewController` / the app-delegate
///   open-URL path respectively. `open_external` is **implemented (Step 6)**
///   via `platform::ios_external::open_url` (`UIApplication.openURL`), so the
///   core `open_url` API now works on iOS like on Android. `file_dialog` is
///   **implemented (Step 7)** via `platform::ios_file::{open_file, save_as}`
///   (`UIDocumentPickerViewController` + `UIDocumentPickerDelegate`), so the
///   core `open_file`/`save_as` APIs now work on iOS like on Android.
///   `receive_files` is **implemented (Step 8)** via
///   `platform::ios_receive` — `application:openURL:options:` is injected onto
///   winit's `WinitApplicationDelegate` (via `class_addMethod`), so files shared
///   into the app reach the `set_shared_files_handler` hook. Warm-start shares
///   are caught; cold-start shares are missed (the method is installed on the
///   main queue *after* `UIApplicationMain` registers the class — see
///   `docs/ios.md` Step 8 for the gap and the patch-winit alternative).
/// - `virtual_keyboard` / `ime` — **implemented (Step 5)**: a hidden
///   `UITextField` first-responder drives the system soft keyboard, and its
///   `UITextFieldDelegate` re-injects edits as `Event::Ime` (`Commit` /
///   `Delete`; `Preedit` composition is best-effort, hence `ime` is reported
///   `true` with CJK marked-text still partial — see `docs/ios.md` §Step 5).
/// - `safe_area` — **implemented (Step 9)**: `EmbedderWindow::safe_area`
///   reads `UIView.safeAreaInsets` from the key window and converts points to
///   physical pixels via `UIScreen.scale`. The `safe_area` capability is now
///   `true` on iOS. Note: there is still **no layout consumer** of these insets
///   in Floem core yet (same gap as Android, which also reports `safe_area:
///   true`) — a follow-up core change must surface `safe_area()` to layout so
///   content clears the notch / Home Indicator.
/// - `accessibility` — **implemented (Phase 4 Step 3)**: a VoiceOver bridge
///   (`accessibility/backend_ios.rs`) subclasses `UIAccessibilityElement` per
///   Floem a11y node, mirrors the neutral tree under the window's `UIView`, and
///   routes VoiceOver actions (activate / increment / decrement / scroll) back
///   to core through the same `PendingAction` channel the desktop backends
///   use. The capability is now `true` on iOS.
/// - `drag_drop` — by design `false` (the winit-ios backend emits no OS file
///   drag-and-drop; Floem's in-app drag works unchanged, as on Android).
///
/// Already enabled: `context_menu` (in-UI renderer, no muda iOS backend —
/// same as Android), `gpu_surface` (Metal, stable, no surface-lifecycle
/// rebuild), `display_info`, `cursor`, `timers`.
pub fn ios_capabilities() -> BackendCapabilities {
    BackendCapabilities {
        clipboard: true,
        cursor: true,
        timers: true,
        file_dialog: true,
        context_menu: true,
        window_title: false,
        accessibility: true,
        system_theme: false,
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

/// iOS platform services (clipboard, theme, external URLs, file dialogs, …).
///
/// The clipboard bridge (`UIPasteboard`) landed in Step 4 and is delegated to
/// `platform::clipboard::Clipboard` (the same path the core `Clipboard` API
/// uses). The remaining `objc2`/`objc2-ui-kit` bridges (external URL, file
/// dialogs, shared files) land in Steps 6–8 (see `docs/ios.md` §7).
pub struct IosServices {}

impl IosServices {
    /// Create the iOS services backend.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for IosServices {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformServices for IosServices {
    fn capabilities(&self) -> BackendCapabilities {
        ios_capabilities()
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

/// iOS embedder window handle.
///
/// Redraws are driven by winit's own redraw-request mechanism on iOS, so
/// [`EmbedderWindow::request_redraw`] is a no-op here; the window id is tracked
/// so backends can correlate protocol events.
pub struct IosEmbedderWindow {
    id: EmbedderWindowId,
}

impl IosEmbedderWindow {
    /// Create a window handle for the given protocol window id.
    pub fn new(id: EmbedderWindowId) -> Self {
        Self { id }
    }
}

impl EmbedderWindow for IosEmbedderWindow {
    fn id(&self) -> EmbedderWindowId {
        self.id
    }

    fn request_redraw(&mut self) {
        // On iOS, winit owns the redraw cycle; nothing to do here.
    }

    fn set_title(&mut self, _title: &str) {
        // iOS has no window title bar; ignore.
    }

    #[allow(deprecated)]
    // `keyWindow` and `UIScreen::mainScreen` are deprecated (multi-scene), but
    // this is a single-scene app (Info.plist `UIApplicationSupportsMultipleScenes
    // = false`), so they are correct here.
    fn safe_area(&self) -> PhysicalInsets<u32> {
        // Safe-area insets live on the live UIWindow's `UIView` and are only
        // readable on the main thread. Read them from the key window.
        let Some(mtm) = MainThreadMarker::new() else {
            return PhysicalInsets { top: 0, left: 0, bottom: 0, right: 0 };
        };
        let app = UIApplication::sharedApplication(mtm);
        let window = match app.keyWindow() {
            Some(w) => w,
            None => return PhysicalInsets { top: 0, left: 0, bottom: 0, right: 0 },
        };
        // `UIView.safeAreaInsets` (UIEdgeInsets, in points).
        let insets: UIEdgeInsets = window.safeAreaInsets();
        // Convert points -> physical pixels via the device scale factor.
        let scale = UIScreen::mainScreen(mtm).scale() as f64;
        let to_px = |v: f64| (v * scale).round() as u32;
        PhysicalInsets {
            top: to_px(insets.top as f64),
            left: to_px(insets.left as f64),
            bottom: to_px(insets.bottom as f64),
            right: to_px(insets.right as f64),
        }
    }
}

/// Read the current safe-area insets from the live key window.
///
/// Used by the iOS event path to emit [`EmbedderEvent::SafeArea`] when the
/// surface resizes (rotation / orientation change), so the core layout can
/// inset content away from the notch / Home Indicator. The window id is
/// irrelevant to the read — `safe_area` queries the global key window — so a
/// throwaway id is fine.
///
/// [`EmbedderEvent::SafeArea`]: crate::embedder::EmbedderEvent::SafeArea
pub fn read_safe_area() -> PhysicalInsets<u32> {
    IosEmbedderWindow::new(0).safe_area()
}

/// iOS rendering surface descriptor holder.
///
/// Constructed from the native window geometry by the integration layer that
/// owns the `UIView`/`CAMetalLayer`.
pub struct IosSurfaceHandle {
    descriptor: SurfaceDescriptor,
}

impl IosSurfaceHandle {
    /// Create a surface handle wrapping `descriptor`.
    pub fn new(descriptor: SurfaceDescriptor) -> Self {
        Self { descriptor }
    }
}

impl SurfaceHandle for IosSurfaceHandle {
    fn descriptor(&self) -> SurfaceDescriptor {
        self.descriptor.clone()
    }
}

/// Build a [`SurfaceDescriptor`] for an iOS GPU (Metal) surface.
///
/// `pixel_size` is the native window's pixel dimensions and `scale` the device
/// scale factor. iOS uses an opaque (`transparent = false`) GPU surface backed
/// by a `CAMetalLayer`.
pub fn ios_surface_descriptor(
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
