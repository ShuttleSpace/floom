/// Neutral props shared by every design system's `Footer`.
///
/// A footer is a bottom application bar (sibling to `AppBar`/`Toolbar`) that
/// holds copyright, links or status content. It sits at the bottom of the
/// layout shell.
#[derive(Debug, Clone)]
pub struct FooterProps {
    /// Bar height in dp.
    pub height: f32,
    /// Elevation / shadow depth of the bar.
    pub elevation: u8,
    /// Draw a top border separator.
    pub bordered: bool,
    /// Horizontal padding inside the bar.
    pub pad: f32,
}

impl Default for FooterProps {
    fn default() -> Self {
        Self {
            height: 56.0,
            elevation: 0,
            bordered: true,
            pad: 16.0,
        }
    }
}
