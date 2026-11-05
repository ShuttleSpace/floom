// SPDX-License-Identifier: MIT
//! Window contract.

use floom_types::{CursorStyle, DisplayInfo, Point, Size, SurfaceId, WindowId};

use crate::surface::{SurfaceConfig, SurfaceTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WindowState {
    #[default]
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowConfig {
    pub title: String,
    pub size: Size,
    pub resizable: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "Floom".into(),
            size: Size::new(800.0, 600.0),
            resizable: true,
        }
    }
}

/// A runtime change to a window that already exists.
///
/// This is the capability Flutter's `window_manager` names, minus the parts
/// that are decided at creation (`WindowConfig`): the app asks, the backend
/// forwards to its window manager.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowCommand {
    /// Logical size of the window's surface — the client area, not the frame.
    Resize(Size),
    /// Position of the window's outer top-left corner, in logical pixels.
    Move(Point),
    Fullscreen(bool),
    Minimized(bool),
    Maximized(bool),
    /// Keep this window above (or back at the normal level of) other windows.
    AlwaysOnTop(bool),
    Decorations(bool),
    Visible(bool),
    /// Raise and give the window keyboard focus.
    Focus,
}

pub trait Window {
    fn id(&self) -> WindowId;
    fn title(&self) -> &str;
    fn size(&self) -> Size;
    fn state(&self) -> WindowState;
    fn set_title(&mut self, title: &str);
    fn request_redraw(&mut self);
    /// Asks the host to show `cursor` over this window.
    ///
    /// A default no-op, because a platform that cannot set a pointer shape must
    /// not become unclickable-looking: a control that asks for `Pointer` and
    /// silently gets nothing should look the same as one that never asked, not
    /// like a broken button. Backends that can, override it.
    fn set_cursor(&mut self, cursor: CursorStyle) {
        let _ = cursor;
    }
    fn surface_id(&self) -> Option<SurfaceId>;
    fn create_surface(&mut self, config: SurfaceConfig) -> SurfaceId;
    fn display_info(&self) -> DisplayInfo;

    /// Apply a runtime change.
    ///
    /// The default refusal is the honest one for a backend that only ever has
    /// the window it was given (a terminal cell grid, a browser canvas): it
    /// has no window manager to ask. A backend that can, overrides it, and
    /// `Ok` means the windowing system accepted the request — the window's
    /// actual size after a resize arrives through the usual resize event, not
    /// through this return value.
    fn control(&mut self, command: WindowCommand) -> Result<(), String> {
        let _ = command;
        Err(format!(
            "window control: the {} backend has no window manager",
            std::env::consts::OS
        ))
    }

    /// Native handles for the window's current surface, if it has one and the
    /// backend can expose them.
    ///
    /// A renderer backend calls this when it needs to build a swap-chain. It
    /// returns `None` for headless/in-memory backends, which is how the app
    /// runner falls back to an offscreen presenter.
    fn surface_target(&self) -> Option<SurfaceTarget> {
        None
    }
}
