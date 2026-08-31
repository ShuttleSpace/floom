//! Capability reporting for embedder backends.
//!
//! Floem queries [`BackendCapabilities`] at startup (and when a window is
//! created) to decide whether a feature is available, downgraded, or a no-op.
//! Unsupported functionality must degrade explicitly instead of failing
//! implicitly.

/// Declares which platform services and surfaces a backend supports.
///
/// This is the contract that lets `floem::launch` keep working on a minimal
/// headless backend while a full winit backend advertises everything.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BackendCapabilities {
    /// System clipboard read/write.
    pub clipboard: bool,
    /// Custom cursor shapes.
    pub cursor: bool,
    /// Wakeups and timers driven by the backend event loop.
    pub timers: bool,
    /// Native open/save file dialogs.
    pub file_dialog: bool,
    /// Context menus.
    pub context_menu: bool,
    /// Window title control.
    pub window_title: bool,
    /// Accessibility tree updates and actions.
    ///
    /// Set via [`BackendCapabilities::with_accessibility`]. Floem's neutral
    /// a11y protocol lives in [`crate::accessibility`]; a backend advertises
    /// support here once it bridges that protocol to a native screen reader.
    pub accessibility: bool,
    /// System theme reporting.
    pub system_theme: bool,
    /// Display/DPI information.
    pub display_info: bool,
    /// Safe area / insets (notches, gesture bars).
    pub safe_area: bool,
    /// Virtual (on-screen) keyboard control.
    pub virtual_keyboard: bool,
    /// Opening external URLs / apps.
    pub open_external: bool,
    /// Receiving files shared into the app from another app (e.g. Android
    /// `ACTION_SEND` / `ACTION_VIEW` intents). Delivered as opaque URIs
    /// (`content://` on Android) rather than filesystem paths.
    pub receive_files: bool,
    /// Drag and drop.
    pub drag_drop: bool,
    /// Text input method editor support.
    pub ime: bool,
    /// GPU-accelerated rendering surface.
    pub gpu_surface: bool,
    /// CPU software rendering surface.
    pub software_surface: bool,
    /// Headless (no real platform window) operation.
    pub headless: bool,
}

impl BackendCapabilities {
    /// Capabilities reported by the default winit desktop/web backend.
    pub fn winit_default() -> Self {
        Self {
            clipboard: true,
            cursor: true,
            timers: true,
            file_dialog: true,
            context_menu: true,
            window_title: true,
            #[cfg(any(
                target_os = "macos",
                target_os = "linux",
                target_os = "windows"
            ))]
            accessibility: true,
            #[cfg(not(any(
                target_os = "macos",
                target_os = "linux",
                target_os = "windows",
                target_env = "ohos"
            )))]
            accessibility: false,
            system_theme: true,
            display_info: true,
            safe_area: false,
            virtual_keyboard: false,
            open_external: false,
            receive_files: false,
            drag_drop: true,
            ime: true,
            gpu_surface: true,
            software_surface: true,
            headless: false,
        }
    }

    /// Capabilities for the headless proof-of-concept backend.
    ///
    /// Everything that requires a real platform window degrades to a no-op.
    pub fn headless() -> Self {
        Self {
            headless: true,
            ..Self::default()
        }
    }

    /// Builder: advertise accessibility tree support. Backends that bridge
    /// [`crate::accessibility`] to a native screen reader set this.
    pub fn with_accessibility(mut self) -> Self {
        self.accessibility = true;
        self
    }
}
