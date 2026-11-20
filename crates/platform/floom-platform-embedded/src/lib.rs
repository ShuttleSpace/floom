// SPDX-License-Identifier: MIT
//! The embedded platform: a window onto a framebuffer, and an event queue.
//!
//! An embedded target has no window manager and no GPU. It has a rectangle of
//! memory that the hardware scans out, and a way to receive input. This crate
//! is the platform half of that: it owns the framebuffer, hands out a window
//! whose surface is that memory, and takes input events from whoever is
//! driving it.
//!
//! # Why this is a sibling crate and not a module in `floom-platform`
//!
//! `floom-platform` is the OS-neutral ABI crate, and the architecture gate
//! holds it to "no concrete OS dependency". An embedded backend that owns a
//! framebuffer is not OS-neutral — it is a concrete backend, and it belongs
//! beside the desktop, terminal and web ones.
//!
//! # What is real here, and what is not
//!
//! The framebuffer is real: pixels land in it, and the software rasteriser
//! draws into it. The window is real: it has a size, a title, a surface and
//! a focus state. The input is real in the sense that matters — events are
//! delivered to the app in order, with the window they belong to.
//!
//! What is *not* here is a driver: nothing scans the framebuffer out to a
//! panel, because that is the board's job and the board is not this crate's
//! business. The host pushes events in and reads pixels out.

use std::collections::VecDeque;
use std::time::Instant;

use floom_platform::{
    DisplayInfo, Platform, PlatformCapabilities, PlatformEvent, Surface, SurfaceConfig, SurfaceId,
    Window, WindowConfig, WindowState,
};
use floom_types::{Modifiers, PlatformId, Point, ScaleFactor, Size, WindowId};

/// The platform id an embedded backend answers with.
///
/// The number is the same one the stand-in in `floom-platform` used, so a
/// host that switches on the platform id keeps working.
pub const EMBEDDED_PLATFORM_ID: PlatformId = PlatformId::new(30, 1);

/// An in-memory surface: the pixels a frame is drawn into.
///
/// There is no native handle, and there is not going to be one — a framebuffer
/// is memory the host already owns. [`Surface::target`] says so, which is how
/// a GPU presenter knows to refuse it and a software presenter knows it may
/// draw.
#[derive(Debug, Clone)]
pub struct EmbeddedSurface {
    id: SurfaceId,
    size: Size,
    scale: f64,
    pixels: Vec<u8>,
}

impl EmbeddedSurface {
    /// A surface of `size` logical units at `scale` device pixels per unit.
    pub fn new(id: SurfaceId, size: Size, scale: f64) -> Self {
        let scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        let (width, height) = physical_size(size, scale);
        Self {
            id,
            size,
            scale,
            pixels: vec![0; (width as usize) * (height as usize) * 4],
        }
    }

    /// The device-pixel extent of the surface.
    pub fn extent(&self) -> (u32, u32) {
        physical_size(self.size, self.scale)
    }

    /// The pixels, four bytes each, RGBA8, top row first.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// The pixels, mutably, for a host that writes into them directly.
    pub fn pixels_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    /// One pixel, by device coordinates.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        let (width, _) = self.extent();
        if x >= width {
            return None;
        }
        let index = ((y as usize) * (width as usize) + (x as usize)) * 4;
        let end = index + 4;
        if end > self.pixels.len() {
            return None;
        }
        Some([
            self.pixels[index],
            self.pixels[index + 1],
            self.pixels[index + 2],
            self.pixels[index + 3],
        ])
    }

    /// Copy a software-rasterised frame in.
    ///
    /// The embedded path has no swap-chain: the software presenter owns a CPU
    /// buffer, and this is the step that puts it on the "screen". The frame
    /// must be the surface's own device-pixel extent; a frame of a different
    /// size is refused rather than copied in half, because a half-copied
    /// frame is a picture nobody can read.
    pub fn present(&mut self, rgba: &[u8]) -> bool {
        let (width, height) = self.extent();
        let expected = (width as usize) * (height as usize) * 4;
        if rgba.len() != expected {
            return false;
        }
        self.pixels.copy_from_slice(rgba);
        true
    }
}

impl Surface for EmbeddedSurface {
    fn id(&self) -> SurfaceId {
        self.id
    }

    fn size(&self) -> Size {
        self.size
    }

    fn resize(&mut self, size: Size) {
        self.size = size;
        let (width, height) = physical_size(size, self.scale);
        let wanted = (width as usize) * (height as usize) * 4;
        if self.pixels.len() != wanted {
            self.pixels.resize(wanted, 0);
        }
    }
}

/// The window an embedded target shows.
///
/// It is a window in the sense that matters: it has an identity, a size, a
/// title and a surface. What it does not have is a window manager to ask, so
/// [`Window::control`] refuses — the same honest refusal the terminal backend
/// makes.
#[derive(Debug)]
pub struct EmbeddedWindow {
    id: WindowId,
    title: String,
    size: Size,
    scale: f64,
    state: WindowState,
    surface: Option<EmbeddedSurface>,
}

impl EmbeddedWindow {
    fn new(id: WindowId, config: WindowConfig, scale: f64) -> Self {
        Self {
            id,
            title: config.title,
            size: config.size,
            scale,
            state: WindowState::Normal,
            surface: None,
        }
    }
    /// The surface, once one has been created.
    pub fn surface(&self) -> Option<&EmbeddedSurface> {
        self.surface.as_ref()
    }

    /// The surface, mutably.
    pub fn surface_mut(&mut self) -> Option<&mut EmbeddedSurface> {
        self.surface.as_mut()
    }
}

impl Window for EmbeddedWindow {
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
        self.surface.as_ref().map(Surface::id)
    }

    fn create_surface(&mut self, config: SurfaceConfig) -> SurfaceId {
        let id = SurfaceId::new(1, 1);
        let mut surface = EmbeddedSurface::new(id, config.size, self.scale);
        if let Some(existing) = &self.surface {
            surface.pixels_mut().copy_from_slice(existing.pixels());
        }
        self.surface = Some(surface);
        id
    }

    fn display_info(&self) -> DisplayInfo {
        DisplayInfo {
            logical_size: self.size,
            scale_factor: ScaleFactor::new(self.scale),
        }
    }
}

/// The embedded platform: a window onto a framebuffer, and an event queue.
///
/// The host owns the board; this owns the memory the board scans out, and
/// the queue the host pushes input into.
#[derive(Debug)]
pub struct EmbeddedPlatform {
    window: Option<EmbeddedWindow>,
    events: VecDeque<PlatformEvent>,
    started: Instant,
    scale: f64,
    redraw_requested: bool,
    exit_requested: bool,
    max_frames: Option<u64>,
    frames: u64,
}

impl EmbeddedPlatform {
    /// A platform with no window yet.
    ///
    /// The window's size comes from the [`WindowConfig`] that creates it, so
    /// the platform does not carry a size of its own.
    pub fn new() -> Self {
        Self {
            window: None,
            events: VecDeque::new(),
            started: Instant::now(),
            scale: 1.0,
            redraw_requested: false,
            exit_requested: false,
            max_frames: None,
            frames: 0,
        }
    }

    /// A platform with one window of `width` × `height` device pixels, and a
    /// framebuffer to match.
    pub fn with_framebuffer(width: u32, height: u32) -> Self {
        let mut platform = Self::new();
        platform.create_window(WindowConfig {
            title: "Floom".into(),
            size: Size::new(width as f64, height as f64),
            resizable: true,
        });
        platform
    }

    /// The device-pixel scale the window's surface is created at.
    pub fn set_scale(&mut self, scale: f64) {
        if scale.is_finite() && scale > 0.0 {
            self.scale = scale;
        }
    }

    /// The window, once one has been created.
    pub fn window(&self) -> Option<&EmbeddedWindow> {
        self.window.as_ref()
    }

    /// The framebuffer the window draws into.
    pub fn framebuffer(&self) -> Option<&EmbeddedSurface> {
        self.window.as_ref().and_then(EmbeddedWindow::surface)
    }

    /// The framebuffer, mutably.
    pub fn framebuffer_mut(&mut self) -> Option<&mut EmbeddedSurface> {
        self.window.as_mut().and_then(EmbeddedWindow::surface_mut)
    }

    /// Push an event into the queue the app will drain.
    pub fn push_event(&mut self, event: PlatformEvent) {
        self.events.push_back(event);
    }

    /// Move the pointer, in logical units.
    pub fn pointer_moved(&mut self, position: Point) {
        if let Some(window) = self.window.as_ref() {
            let id = window.id();
            self.push_event(PlatformEvent::PointerMoved {
                window: id,
                position,
                modifiers: Modifiers::default(),
            });
        }
    }

    /// Press or release a pointer button, in logical units.
    pub fn pointer_button(
        &mut self,
        position: Point,
        button: floom_platform::PointerButton,
        pressed: bool,
    ) {
        if let Some(window) = self.window.as_ref() {
            let id = window.id();
            self.push_event(PlatformEvent::PointerButton {
                window: id,
                position,
                button,
                pressed,
                modifiers: Modifiers::default(),
            });
        }
    }

    /// Resize the window, and with it the framebuffer.
    ///
    /// The framebuffer follows the window: a framebuffer of the wrong size is
    /// a frame of the wrong size, and the pixels would be read as if they
    /// were the surface's.
    pub fn resize(&mut self, size: Size) {
        if let Some(window) = self.window.as_mut() {
            window.size = size;
            if let Some(surface) = window.surface.as_mut() {
                surface.resize(size);
            }
            let id = window.id();
            self.events
                .push_back(PlatformEvent::WindowResized { window: id, size });
        }
    }

    /// Whether a redraw was asked for since the last [`Self::take_redraw`].
    pub fn redraw_requested(&self) -> bool {
        self.redraw_requested
    }

    /// Take the redraw flag, clearing it.
    pub fn take_redraw(&mut self) -> bool {
        std::mem::take(&mut self.redraw_requested)
    }

    /// Ask the run loop to stop.
    ///
    /// A board that has nothing left to show says so here, the way a desktop
    /// app closes its window. The run loop observes it and returns.
    pub fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    /// Whether [`Self::request_exit`] has been called.
    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }

    /// Stop after `frames` frames.
    ///
    /// An embedded target runs until its board says stop, which is no use to
    /// a test that needs the loop to end. A frame limit is that: the loop
    /// serves exactly so many frames, and everything drawn up to then is
    /// still in the framebuffer.
    pub fn set_max_frames(&mut self, frames: u64) {
        self.max_frames = Some(frames);
    }

    /// Frames served so far.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// Count one frame, and report whether the loop should keep going.
    ///
    /// The run loop calls this once per frame. The limit is inclusive: a
    /// limit of three serves three frames.
    pub fn count_frame(&mut self) -> bool {
        self.frames += 1;
        !self.exit_requested && self.max_frames.is_none_or(|max| self.frames <= max)
    }
}

impl Default for EmbeddedPlatform {
    /// Same as [`new`](Self::new): a platform with no window yet.
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for EmbeddedPlatform {
    fn id(&self) -> PlatformId {
        EMBEDDED_PLATFORM_ID
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities::embedded_default()
    }

    fn create_window(&mut self, config: WindowConfig) -> WindowId {
        let id = WindowId::new(1, 1);
        let size = config.size;
        let mut window = EmbeddedWindow::new(id, config, self.scale);
        // A surface exists from the start: an embedded target has memory
        // whether or not anybody has drawn into it, and a window with no
        // surface would leave the renderer with nothing to attach to.
        let surface_id = SurfaceId::new(1, 1);
        window.surface = Some(EmbeddedSurface::new(surface_id, size, self.scale));
        self.window = Some(window);
        self.events.push_back(PlatformEvent::WindowCreated {
            window: id,
            surface: surface_id,
            size,
        });
        id
    }

    fn window(&self, id: WindowId) -> Option<&dyn Window> {
        self.window
            .as_ref()
            .filter(|window| window.id() == id)
            .map(|window| window as &dyn Window)
    }

    fn window_mut(&mut self, id: WindowId) -> Option<&mut dyn Window> {
        self.window
            .as_mut()
            .filter(|window| window.id() == id)
            .map(|window| window as &mut dyn Window)
    }

    fn poll_events(&mut self, out: &mut Vec<PlatformEvent>) {
        out.extend(self.events.drain(..));
    }

    fn request_frame(&mut self, window: WindowId) {
        self.redraw_requested = true;
        // The runner draws on a `RedrawRequested` *event*, not on a flag: a
        // poll-driven backend has no window system to emit one, so the
        // backend that asks for the frame has to queue it. Without this the
        // app is asked to redraw and never draws.
        self.events
            .push_back(PlatformEvent::RedrawRequested { window });
    }

    fn now_secs(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }
}

/// The device-pixel extent of a logical size at a scale.
fn physical_size(size: Size, scale: f64) -> (u32, u32) {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    (
        (size.width.max(0.0) * scale).ceil() as u32,
        (size.height.max(0.0) * scale).ceil() as u32,
    )
}
