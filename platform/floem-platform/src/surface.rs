//! Surface / render-target contract (logical, no GPU types).

use floem_types::{Size, SurfaceId};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceConfig {
    pub size: Size,
    pub transparent: bool,
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self {
            size: Size::new(800.0, 600.0),
            transparent: false,
        }
    }
}

pub trait Surface {
    fn id(&self) -> SurfaceId;
    fn size(&self) -> Size;
    fn resize(&mut self, size: Size);
}
