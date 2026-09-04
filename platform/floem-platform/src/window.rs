//! Window contract.

use floem_types::{DisplayInfo, Size, SurfaceId, WindowId};

use crate::surface::SurfaceConfig;

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
            title: "Floem".into(),
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
    fn surface_id(&self) -> Option<SurfaceId>;
    fn create_surface(&mut self, config: SurfaceConfig) -> SurfaceId;
    fn display_info(&self) -> DisplayInfo;
}
