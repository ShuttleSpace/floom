//! Window contract.

use floom_types::{CursorStyle, DisplayInfo, Size, SurfaceId, WindowId};

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
