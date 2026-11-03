//! Desktop platform adapter.
//!
//! Maps Winit-style window / pointer / keyboard / IME lifecycle onto the
//! `floom-platform` ABI without pulling Winit into `floom-platform` itself.
//! Real `floem-winit` wiring stays in the app/runtime layer; this crate owns
//! the ABI-facing desktop services (clipboard + IME stubs included).

pub mod accessibility;

pub mod native;

#[cfg(target_os = "macos")]
pub mod native_appkit;

/// The real macOS clipboard (`NSPasteboard`). Split out because it is the one
/// part of this adapter that must reach AppKit directly: the pasteboard belongs
/// to the session, not to a window.
///
/// Gated on `winit` because [`crate::winit_backend`] is its only caller: the
/// headless [`crate::DesktopPlatform`] keeps the in-process placeholder, so
/// compiling this module without `winit` would produce dead code that no build
/// could reach.
#[cfg(all(target_os = "macos", feature = "winit"))]
pub mod clipboard;

#[cfg(feature = "winit")]
pub mod winit_backend;

#[cfg(feature = "winit")]
pub mod winit_input;

#[cfg(feature = "winit")]
pub use winit_backend::{WinitApp, WinitPlatform, run};

#[cfg(target_os = "macos")]
pub use native_appkit::{
    MacNativeViewHost, NativeParent, NativeViewUnavailable, ViewFactory, ViewFrame,
};

pub use native::{
    DesktopNativeBridge, NativeViewHost, Origin, ViewFrames, ViewPlacement, desktop_capabilities,
    device_to_points, mirror_device, os_frame, split_placement, to_view_placement,
};

use std::collections::HashMap;
use std::time::Instant;

use floom_platform::{
    Clipboard, Ime, KeyCode, Modifiers, Platform, PlatformCapabilities, PlatformEvent,
    PointerButton, SurfaceConfig, Window, WindowCommand, WindowConfig, WindowState,
};
use floom_types::{DisplayInfo, PlatformId, Point, ScaleFactor, Size, SurfaceId, WindowId};

/// Logical desktop input sample (Winit-equivalent, OS-handle free).
#[derive(Debug, Clone, PartialEq)]
pub enum DesktopInput {
    PointerMoved {
        position: Point,
        modifiers: Modifiers,
    },
    PointerButton {
        position: Point,
        button: PointerButton,
        pressed: bool,
        modifiers: Modifiers,
    },
    Scroll {
        position: Point,
        delta_x: f64,
        delta_y: f64,
        modifiers: Modifiers,
    },
    Key {
        code: KeyCode,
        pressed: bool,
        /// Auto-repeat from a held key.
        repeat: bool,
        modifiers: Modifiers,
    },
    ModifiersChanged(Modifiers),
    ImePreedit {
        text: String,
    },
    ImeCommit {
        text: String,
    },
    Focused(bool),
    Resized(Size),
    CloseRequested,
    Redraw,
}

#[derive(Debug)]
struct DesktopSurface {
    id: SurfaceId,
    size: Size,
}

#[derive(Debug)]
struct DesktopWindow {
    id: WindowId,
    title: String,
    size: Size,
    state: WindowState,
    surface: Option<DesktopSurface>,
    next_surface: u32,
    scale: ScaleFactor,
}

impl Window for DesktopWindow {
    fn id(&self) -> WindowId {
        self.id
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn size(&self) -> Size {
        self.size
    }

    fn state(&self) -> WindowState {
        self.state
    }

    fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
    }

    fn request_redraw(&mut self) {}

    fn surface_id(&self) -> Option<SurfaceId> {
        self.surface.as_ref().map(|s| s.id)
    }

    fn create_surface(&mut self, config: SurfaceConfig) -> SurfaceId {
        let id = SurfaceId::new(self.next_surface, 1);
        self.next_surface += 1;
        self.surface = Some(DesktopSurface {
            id,
            size: config.size,
        });
        id
    }

    fn display_info(&self) -> DisplayInfo {
        DisplayInfo {
            logical_size: self.size,
            scale_factor: self.scale,
        }
    }

    /// The reference backend has no window manager, so it records what a real
    /// one would have applied. That is enough for the parts a test can observe
    /// (`size`, `state`), and the rest is accepted and forgotten.
    fn control(&mut self, command: WindowCommand) -> Result<(), String> {
        match command {
            WindowCommand::Resize(size) => self.size = size,
            WindowCommand::Fullscreen(true) => self.state = WindowState::Fullscreen,
            WindowCommand::Minimized(true) => self.state = WindowState::Minimized,
            WindowCommand::Maximized(true) => self.state = WindowState::Maximized,
            WindowCommand::Fullscreen(false)
            | WindowCommand::Minimized(false)
            | WindowCommand::Maximized(false) => self.state = WindowState::Normal,
            WindowCommand::Move(_)
            | WindowCommand::AlwaysOnTop(_)
            | WindowCommand::Decorations(_)
            | WindowCommand::Visible(_)
            | WindowCommand::Focus => {}
        }
        Ok(())
    }
}

/// Desktop [`Platform`] with clipboard + IME.
#[derive(Debug)]
pub struct DesktopPlatform {
    id: PlatformId,
    windows: HashMap<u32, DesktopWindow>,
    next_window: u32,
    events: Vec<PlatformEvent>,
    clipboard: String,
    started: Instant,
    ime_enabled: HashMap<u32, bool>,
}

impl Default for DesktopPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopPlatform {
    pub fn new() -> Self {
        Self {
            id: PlatformId::new(2, 1),
            windows: HashMap::new(),
            next_window: 0,
            events: Vec::new(),
            clipboard: String::new(),
            started: Instant::now(),
            ime_enabled: HashMap::new(),
        }
    }

    /// Inject a Winit-mapped input event for `window`.
    pub fn dispatch_input(&mut self, window: WindowId, input: DesktopInput) {
        let event = match input {
            DesktopInput::PointerMoved {
                position,
                modifiers,
            } => PlatformEvent::PointerMoved {
                window,
                position,
                modifiers,
            },
            DesktopInput::PointerButton {
                position,
                button,
                pressed,
                modifiers,
            } => PlatformEvent::PointerButton {
                window,
                position,
                button,
                pressed,
                modifiers,
            },
            DesktopInput::Scroll {
                position,
                delta_x,
                delta_y,
                modifiers,
            } => PlatformEvent::Scroll {
                window,
                position,
                delta_x,
                delta_y,
                modifiers,
            },
            DesktopInput::Key {
                code,
                pressed,
                repeat,
                modifiers,
            } => PlatformEvent::Key {
                window,
                code,
                pressed,
                repeat,
                modifiers,
            },
            DesktopInput::ModifiersChanged(modifiers) => {
                PlatformEvent::ModifiersChanged { window, modifiers }
            }
            DesktopInput::ImePreedit { text } => PlatformEvent::ImePreedit { window, text },
            DesktopInput::ImeCommit { text } => PlatformEvent::ImeCommit { window, text },
            DesktopInput::Focused(focused) => PlatformEvent::WindowFocused { window, focused },
            DesktopInput::Resized(size) => {
                if let Some(w) = self.windows.get_mut(&window.index())
                    && w.id == window
                {
                    w.size = size;
                    if let Some(s) = w.surface.as_mut() {
                        s.size = size;
                    }
                }
                PlatformEvent::WindowResized { window, size }
            }
            DesktopInput::CloseRequested => PlatformEvent::WindowClosed { window },
            DesktopInput::Redraw => PlatformEvent::RedrawRequested { window },
        };
        self.events.push(event);
    }

    pub fn ime_is_enabled(&self, window: WindowId) -> bool {
        self.ime_enabled
            .get(&window.index())
            .copied()
            .unwrap_or(false)
    }
}

impl Platform for DesktopPlatform {
    fn id(&self) -> PlatformId {
        self.id
    }

    fn capabilities(&self) -> PlatformCapabilities {
        // No native-view bridge is installed on this platform yet, so the flag
        // stays off — advertising it would make every `NativeSurface` mount fail
        // with "no supported mode". `native::desktop_capabilities` is the single
        // place the flag is derived from the bridge's presence, so step 7's OS
        // half flips this one argument and nothing else.
        desktop_capabilities(false)
    }

    fn create_window(&mut self, config: WindowConfig) -> WindowId {
        let index = self.next_window;
        self.next_window += 1;
        let id = WindowId::new(index, 1);
        let mut window = DesktopWindow {
            id,
            title: config.title,
            size: config.size,
            state: WindowState::Normal,
            surface: None,
            next_surface: 0,
            scale: ScaleFactor::new(1.0),
        };
        let surface = window.create_surface(SurfaceConfig {
            size: config.size,
            transparent: false,
        });
        self.events.push(PlatformEvent::WindowCreated {
            window: id,
            surface,
            size: config.size,
        });
        self.windows.insert(index, window);
        id
    }

    fn window(&self, id: WindowId) -> Option<&dyn Window> {
        self.windows
            .get(&id.index())
            .filter(|w| w.id == id)
            .map(|w| w as &dyn Window)
    }

    fn window_mut(&mut self, id: WindowId) -> Option<&mut dyn Window> {
        let w = self.windows.get_mut(&id.index())?;
        if w.id != id {
            return None;
        }
        Some(w as &mut dyn Window)
    }

    fn poll_events(&mut self, out: &mut Vec<PlatformEvent>) {
        out.append(&mut self.events);
    }

    fn request_frame(&mut self, window: WindowId) {
        self.events.push(PlatformEvent::RedrawRequested { window });
    }

    fn now_secs(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }
}

impl Clipboard for DesktopPlatform {
    fn get_text(&mut self) -> Option<String> {
        Some(self.clipboard.clone())
    }

    fn set_text(&mut self, text: &str) -> bool {
        self.clipboard = text.to_string();
        true
    }
}

impl Ime for DesktopPlatform {
    fn set_enabled(&mut self, window: WindowId, enabled: bool) {
        self.ime_enabled.insert(window.index(), enabled);
    }

    fn set_cursor_area(&mut self, _window: WindowId, _origin: Point, _size: Size) {}
}
