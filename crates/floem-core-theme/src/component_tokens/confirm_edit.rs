#[derive(Debug, Clone, PartialEq)]
pub struct ConfirmEditTokens {
    pub icon_size: f32,
    pub gap: f32,
    pub radius: f32,
}

impl Default for ConfirmEditTokens {
    fn default() -> Self {
        Self {
            icon_size: 20.0,
            gap: 8.0,
            radius: 6.0,
        }
    }
}
