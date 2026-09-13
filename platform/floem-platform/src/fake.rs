//! In-memory fake platform for contract tests (no OS deps).

use std::collections::HashMap;
use std::time::Instant;

use floem_types::{DisplayInfo, PlatformId, Point, ScaleFactor, Size, SurfaceId, WindowId};

use crate::capabilities::{Capability, PlatformCapabilities};
use crate::event::PlatformEvent;
use crate::surface::{Surface, SurfaceConfig};
use crate::window::{Window, WindowConfig, WindowState};
use crate::{Clipboard, Ime, Platform};

#[derive(Debug)]
struct FakeSurface {
    id: SurfaceId,
    size: Size,
}

impl Surface for FakeSurface {
    fn id(&self) -> SurfaceId {
        self.id
    }

    fn size(&self) -> Size {
        self.size
    }

    fn resize(&mut self, size: Size) {
        self.size = size;
    }
}

#[derive(Debug)]
struct FakeWindow {
    id: WindowId,
    title: String,
    size: Size,
    state: WindowState,
    surface: Option<FakeSurface>,
    next_surface: u32,
}

impl Window for FakeWindow {
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
        self.surface = Some(FakeSurface {
            id,
            size: config.size,
        });
        id
    }

    fn display_info(&self) -> DisplayInfo {
        DisplayInfo {
            logical_size: self.size,
            scale_factor: ScaleFactor::new(1.0),
        }
    }
}

/// In-memory host implementing [`Platform`], [`Clipboard`], and [`Ime`].
///
/// Used as the contract test double (`FakePlatform`) and as the headless
/// backend behind mobile/web/embedded adapters until native glue lands.
#[derive(Debug)]
pub struct HostPlatform {
    id: PlatformId,
    caps: PlatformCapabilities,
    windows: HashMap<u32, FakeWindow>,
    next_window: u32,
    events: Vec<PlatformEvent>,
    clipboard: String,
    started: Instant,
    ime_enabled: HashMap<u32, bool>,
}

/// Alias kept for existing contract tests.
pub type FakePlatform = HostPlatform;

impl Default for HostPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl HostPlatform {
    pub fn new() -> Self {
        Self {
            id: PlatformId::new(1, 1),
            caps: PlatformCapabilities::desktop_default(),
            windows: HashMap::new(),
            next_window: 0,
            events: Vec::new(),
            clipboard: String::new(),
            started: Instant::now(),
            ime_enabled: HashMap::new(),
        }
    }

    pub fn with_id(mut self, id: PlatformId) -> Self {
        self.id = id;
        self
    }

    pub fn with_capabilities(mut self, caps: PlatformCapabilities) -> Self {
        self.caps = caps;
        self
    }

    pub fn push_event(&mut self, event: PlatformEvent) {
        self.events.push(event);
    }

    pub fn inject_pointer_click(&mut self, window: WindowId, position: Point) {
        self.events.push(PlatformEvent::PointerButton {
            window,
            position,
            button: crate::PointerButton::Primary,
            pressed: true,
            modifiers: floem_types::Modifiers::NONE,
        });
        self.events.push(PlatformEvent::PointerButton {
            window,
            position,
            button: crate::PointerButton::Primary,
            pressed: false,
            modifiers: floem_types::Modifiers::NONE,
        });
    }

    pub fn suspend(&mut self) {
        self.events.push(PlatformEvent::Suspended);
    }

    pub fn resume(&mut self) {
        self.events.push(PlatformEvent::Resumed);
    }
}

impl Platform for HostPlatform {
    fn id(&self) -> PlatformId {
        self.id
    }

    fn capabilities(&self) -> PlatformCapabilities {
        self.caps
    }

    fn create_window(&mut self, config: WindowConfig) -> WindowId {
        let index = self.next_window;
        self.next_window += 1;
        let id = WindowId::new(index, 1);
        let mut window = FakeWindow {
            id,
            title: config.title.clone(),
            size: config.size,
            state: WindowState::Normal,
            surface: None,
            next_surface: 0,
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

impl Clipboard for HostPlatform {
    fn get_text(&mut self) -> Option<String> {
        if self.caps.supports(Capability::CLIPBOARD) {
            Some(self.clipboard.clone())
        } else {
            None
        }
    }

    fn set_text(&mut self, text: &str) -> bool {
        if !self.caps.supports(Capability::CLIPBOARD) {
            return false;
        }
        self.clipboard = text.to_string();
        true
    }
}

impl Ime for HostPlatform {
    fn set_enabled(&mut self, window: WindowId, enabled: bool) {
        if self.caps.supports(Capability::IME) {
            self.ime_enabled.insert(window.index(), enabled);
        }
    }

    fn set_cursor_area(&mut self, _window: WindowId, _origin: Point, _size: Size) {}
}
