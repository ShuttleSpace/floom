#[derive(Debug, Clone, PartialEq)]
pub struct ListGroupTokens {
    /// Left indent applied to child items when the group is inset in px.
    pub indent: f32,
    /// Height of the group header row in px.
    pub header_height: f32,
    /// Inner padding of the items area in px.
    pub content_padding: f32,
    /// Vertical gap between items in px.
    pub gap: f32,
}

impl Default for ListGroupTokens {
    fn default() -> Self {
        Self {
            indent: 16.0,
            header_height: 48.0,
            content_padding: 8.0,
            gap: 4.0,
        }
    }
}
