#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadiusTokens {
    pub none: f32,
    pub xs: f32,
    pub sm: f32,
    pub md: f32,
    pub lg: f32,
    pub xl: f32,
    pub full: f32,
}
impl Default for RadiusTokens {
    fn default() -> Self {
        Self {
            none: 0.0,
            xs: 4.0,
            sm: 8.0,
            md: 12.0,
            lg: 16.0,
            xl: 24.0,
            full: 9999.0,
        }
    }
}
