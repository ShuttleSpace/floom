// SPDX-License-Identifier: MIT
//! In-memory fake platform for contract tests (no OS deps).

use std::collections::HashMap;
use std::time::Instant;

use floom_types::{
    CursorStyle, DisplayInfo, PlatformId, Point, ScaleFactor, Size, SurfaceId, WindowId,
};

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
    /// Device pixels per logical pixel, as the display reports it. Real windows
    /// are created on whichever display they land on, so this is a property of
    /// the window and not a constant of the backend.
    scale: f64,
    state: WindowState,
    surface: Option<FakeSurface>,
    next_surface: u32,
    /// The last cursor the host asked for.
    ///
    /// Recorded rather than applied, so a test can assert on what a control
    /// resolved to. This is the same trick `ime_cursor_area` plays for the
    /// composition caret.
    cursor: Option<CursorStyle>,
    /// How many times `set_cursor` was called, whatever the value.
    ///
    /// Counting separately from `cursor` is the point: a runner that pushes the
    /// same answer on every pointer move leaves `cursor` looking correct and is
    /// still doing a platform round trip per motion event. Only a count shows
    /// that.
    cursor_calls: u32,
}

impl Window for FakeWindow {
    fn set_cursor(&mut self, cursor: CursorStyle) {
        self.cursor = Some(cursor);
        self.cursor_calls += 1;
    }

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
            scale_factor: ScaleFactor::new(self.scale),
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
    /// Device-pixel scale handed to windows this backend creates. Lets a test
    /// boot onto a Retina display without a Retina display.
    scale: f64,
    /// Last caret rectangle reported to the IME bridge (`set_cursor_area`),
    /// kept so tests can assert the runner's caret-area handshake fired.
    ime_cursor_area: Option<(Point, Size)>,
}

/// Alias kept for existing contract tests.
pub type FakePlatform = HostPlatform;

impl Default for HostPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl HostPlatform {
    /// A host platform with a window and a soft keyboard.
    ///
    /// The clipboard is **not** claimed: this adapter's clipboard is a
    /// `String` in this process, and a clipboard another process cannot read
    /// is not a clipboard. A test that wants to exercise the capability
    /// gating says so by setting the capabilities.
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
            scale: 1.0,
            ime_cursor_area: None,
        }
    }

    pub fn with_id(mut self, id: PlatformId) -> Self {
        self.id = id;
        self
    }

    /// Create subsequent windows on a display of `scale` device pixels per
    /// logical pixel. Non-finite or non-positive values are ignored, matching
    /// [`ScaleFactor`]'s own contract.
    pub fn with_scale_factor(mut self, scale: f64) -> Self {
        if scale.is_finite() && scale > 0.0 {
            self.scale = scale;
        }
        self
    }

    /// Set the device-pixel scale for *subsequently* created windows (the web
    /// backend seeds `window.devicePixelRatio()` before any window exists).
    pub fn set_scale(&mut self, scale: f64) {
        if scale.is_finite() && scale > 0.0 {
            self.scale = scale;
        }
    }

    /// Move `window` to a display of `scale` and announce the resize.
    ///
    /// A display change keeps the window the same size *on the glass* — the
    /// physical rect is what does not move — so the logical rect the framework
    /// lays out in does. The new logical size arrives as the same
    /// [`PlatformEvent::WindowResized`] every other backend sends, because the
    /// ABI has no separate scale event: the scale is read back off the window.
    pub fn set_scale_factor(&mut self, window: WindowId, scale: f64) {
        if !(scale.is_finite() && scale > 0.0) {
            return;
        }
        let Some(w) = self
            .windows
            .get_mut(&window.index())
            .filter(|w| w.id == window)
        else {
            return;
        };
        let physical = Size::new(w.size.width * w.scale, w.size.height * w.scale);
        w.scale = scale;
        w.size = Size::new(physical.width / scale, physical.height / scale);
        let size = w.size;
        if let Some(surface) = w.surface.as_mut() {
            surface.size = size;
        }
        self.events
            .push(PlatformEvent::WindowResized { window, size });
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
            modifiers: floom_types::Modifiers::NONE,
        });
        self.events.push(PlatformEvent::PointerButton {
            window,
            position,
            button: crate::PointerButton::Primary,
            pressed: false,
            modifiers: floom_types::Modifiers::NONE,
        });
    }

    pub fn suspend(&mut self) {
        self.events.push(PlatformEvent::Suspended);
    }

    pub fn resume(&mut self) {
        self.events.push(PlatformEvent::Resumed);
    }

    /// The last caret rectangle the IME bridge was told about (`set_cursor_area`),
    /// or `None` if no caret-area handshake has happened yet.
    pub fn ime_cursor_area(&self) -> Option<(Point, Size)> {
        self.ime_cursor_area
    }

    /// The cursor the host last asked `window` to show, or `None` if it never
    /// asked.
    ///
    /// `CursorStyle::Default` and "never asked" are different answers and this
    /// keeps them apart: the first is a reset after leaving a control, the second
    /// means nothing under the pointer has ever resolved.
    /// How many times `window` has been told which cursor to show.
    ///
    /// The value in [`Self::cursor`] cannot answer "did the runner skip the
    /// redundant call", because a redundant call leaves it unchanged.
    pub fn cursor_calls(&self, window: WindowId) -> u32 {
        self.windows
            .values()
            .find(|w| w.id == window)
            .map(|w| w.cursor_calls)
            .unwrap_or(0)
    }

    pub fn cursor(&self, window: WindowId) -> Option<CursorStyle> {
        self.windows
            .values()
            .find(|w| w.id == window)
            .and_then(|w| w.cursor)
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
            scale: self.scale,
            state: WindowState::Normal,
            surface: None,
            next_surface: 0,
            cursor: None,
            cursor_calls: 0,
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

    fn ime(&mut self, _window: WindowId) -> Option<&mut dyn Ime> {
        Some(self)
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

    fn set_cursor_area(&mut self, _window: WindowId, origin: Point, size: Size) {
        if self.caps.supports(Capability::IME) {
            self.ime_cursor_area = Some((origin, size));
        }
    }
}
