/// Neutral props shared by every design system's `Toolbar`.
///
/// A toolbar is a compact horizontal bar (taller than a plain AppBar row but
/// shorter than a full AppBar). It carries a leading icon slot, a title and an
/// optional trailing actions slot supplied via the `actions(child)` builder.
#[derive(Debug, Clone)]
pub struct ToolbarProps {
    pub title: String,
    /// Optional leading icon name.
    pub leading: Option<String>,
    /// Elevation / shadow depth of the bar.
    pub elevation: u8,
    /// Bar height in dp.
    pub height: f32,
}

impl Default for ToolbarProps {
    fn default() -> Self {
        Self {
            title: String::new(),
            leading: None,
            elevation: 0,
            height: 48.0,
        }
    }
}
