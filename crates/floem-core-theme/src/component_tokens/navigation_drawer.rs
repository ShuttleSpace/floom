#[derive(Debug, Clone, PartialEq)]
pub struct NavigationDrawerTokens {
    /// Width of the drawer panel in dp.
    pub width: f32,
    /// Default elevation/shadow depth when open.
    pub elevation: u8,
    /// Inner padding inside the drawer in dp.
    pub padding: f32,
}

impl Default for NavigationDrawerTokens {
    fn default() -> Self {
        Self {
            width: 304.0,
            elevation: 4,
            padding: 12.0,
        }
    }
}
