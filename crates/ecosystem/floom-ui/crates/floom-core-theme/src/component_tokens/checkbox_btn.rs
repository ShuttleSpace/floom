#[derive(Debug, Clone, PartialEq)]
pub struct CheckboxBtnTokens {
    /// Height of the toggle button in px.
    pub height: f32,
    /// Inner padding in px.
    pub padding: f32,
    /// Corner radius in px.
    pub radius: f32,
    /// Gap between the check glyph and the label in px.
    pub gap: f32,
    /// Label font size in px.
    pub font_size: f32,
}

impl Default for CheckboxBtnTokens {
    fn default() -> Self {
        Self {
            height: 40.0,
            padding: 12.0,
            radius: 8.0,
            gap: 8.0,
            font_size: 14.0,
        }
    }
}
