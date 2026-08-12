#[derive(Debug, Clone, PartialEq)]
pub struct ListSubheaderTokens {
    /// Subheader font size in px.
    pub font_size: f32,
    /// Height of the subheader row in px.
    pub height: f32,
    /// Left/right padding when not inset in px.
    pub padding_horizontal: f32,
    /// Left/right padding when inset in px.
    pub inset_padding: f32,
    /// Letter spacing in px (small-caps list-subheader look).
    pub letter_spacing: f32,
}

impl Default for ListSubheaderTokens {
    fn default() -> Self {
        Self {
            font_size: 14.0,
            height: 48.0,
            padding_horizontal: 16.0,
            inset_padding: 32.0,
            letter_spacing: 0.1,
        }
    }
}
