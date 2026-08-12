#[derive(Debug, Clone)]
pub struct NavigationDrawerProps {
    /// Whether the drawer is open on first render.
    pub open: bool,
    /// Width of the drawer panel in dp.
    pub width: f32,
    /// Elevation/shadow depth of the drawer when open.
    pub elevation: u8,
}

impl Default for NavigationDrawerProps {
    fn default() -> Self {
        Self {
            open: false,
            width: 304.0,
            elevation: 4,
        }
    }
}
