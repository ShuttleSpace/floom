#[derive(Debug, Clone)]
pub struct WindowProps {
    pub height: f32,
    pub show_arrows: bool,
    pub show_dots: bool,
    pub continuous: bool,
}

impl Default for WindowProps {
    fn default() -> Self {
        Self {
            height: 0.0,
            show_arrows: true,
            show_dots: false,
            continuous: true,
        }
    }
}
