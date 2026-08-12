#[derive(Debug, Clone, PartialEq)]
pub struct BreadcrumbsTokens {
    /// Gap between items and dividers in dp.
    pub gap: f32,
    /// Item font size in dp.
    pub font_size: f32,
    /// Divider string rendered between items.
    pub divider: String,
}

impl Default for BreadcrumbsTokens {
    fn default() -> Self {
        Self {
            gap: 8.0,
            font_size: 14.0,
            divider: "/".to_string(),
        }
    }
}
