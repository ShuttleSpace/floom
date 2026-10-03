/// Edge from which a navigation drawer slides in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationDrawerSide {
    /// Slides in from the left edge of the app.
    Left,
    /// Slides in from the right edge of the app.
    Right,
}

#[derive(Debug, Clone)]
pub struct NavigationDrawerProps {
    /// Whether the drawer is open on first render.
    pub open: bool,
    /// Width of the drawer panel in dp.
    pub width: f32,
    /// Elevation/shadow depth of the drawer when open.
    pub elevation: u8,
    /// Edge the drawer slides in from.
    pub side: NavigationDrawerSide,
}

impl Default for NavigationDrawerProps {
    fn default() -> Self {
        Self {
            open: false,
            width: 360.0,
            elevation: 4,
            side: NavigationDrawerSide::Left,
        }
    }
}
