#[derive(Debug, Clone, PartialEq)]
pub struct ItemGroupTokens {
    pub gap: f32,
    pub pad: f32,
    pub radius: f32,
}

impl Default for ItemGroupTokens {
    fn default() -> Self {
        Self {
            gap: 4.0,
            pad: 12.0,
            radius: 8.0,
        }
    }
}
