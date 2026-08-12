#[derive(Debug, Clone)]
pub struct ColorPickerProps {
    pub show_alpha: bool,
}

impl Default for ColorPickerProps {
    fn default() -> Self {
        Self { show_alpha: true }
    }
}
