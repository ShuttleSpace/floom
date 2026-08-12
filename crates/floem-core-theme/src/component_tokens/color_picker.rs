#[derive(Debug, Clone, PartialEq)]
pub struct ColorPickerTokens {
    pub sv_width: f32,
    pub sv_height: f32,
    pub slider_height: f32,
    pub thumb_size: f32,
    pub radius: f32,
}

impl Default for ColorPickerTokens {
    fn default() -> Self {
        Self {
            sv_width: 240.0,
            sv_height: 160.0,
            slider_height: 16.0,
            thumb_size: 16.0,
            radius: 8.0,
        }
    }
}
