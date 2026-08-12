#[derive(Debug, Clone, PartialEq)]
pub struct ExpansionPanelsTokens {
    /// Height of each panel header row in px.
    pub header_height: f32,
    /// Inner padding of the expanded content area in px.
    pub content_padding: f32,
    /// Left indent applied to child content in px.
    pub indent: f32,
    /// Divider / outline border width in px.
    pub border: f32,
    /// Gap between stacked panels in px.
    pub gap: f32,
    /// Panel corner radius in px.
    pub radius: f32,
}

impl Default for ExpansionPanelsTokens {
    fn default() -> Self {
        Self {
            header_height: 48.0,
            content_padding: 16.0,
            indent: 16.0,
            border: 1.0,
            gap: 8.0,
            radius: 12.0,
        }
    }
}
