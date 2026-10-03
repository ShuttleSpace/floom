/// Neutral props shared by every design system's `Footer`.
///
/// A footer is a bottom application bar (sibling to `AppBar`/`Toolbar`) that
/// holds copyright, links or status content. It sits at the bottom of the
/// layout shell.
#[derive(Debug, Clone)]
pub struct FooterProps {
    /// Bar height in dp. `None` defers to the active design system's
    /// `FooterTokens::height` (M3 56 / Cupertino 44 / Fluent 48).
    pub height: Option<f32>,
    /// Elevation / shadow depth of the bar.
    pub elevation: u8,
    /// Draw a top border separator.
    pub bordered: bool,
    /// Horizontal padding inside the bar. `None` defers to the active design
    /// system's `FooterTokens::pad` (M3 16 / Cupertino 12 / Fluent 16).
    pub pad: Option<f32>,
}

impl Default for FooterProps {
    fn default() -> Self {
        Self {
            height: None,
            elevation: 0,
            bordered: true,
            pad: None,
        }
    }
}
