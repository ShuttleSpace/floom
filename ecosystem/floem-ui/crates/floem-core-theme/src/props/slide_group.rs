#[derive(Debug, Clone)]
pub struct SlideGroupProps {
    pub show_arrows: bool,
    pub center_active: bool,
    pub mandatory: bool,
}

impl Default for SlideGroupProps {
    fn default() -> Self {
        Self {
            show_arrows: true,
            center_active: false,
            mandatory: false,
        }
    }
}
