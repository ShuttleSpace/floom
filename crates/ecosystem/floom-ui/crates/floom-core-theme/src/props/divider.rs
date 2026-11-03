#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DividerOrientation {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DividerProps {
    pub orientation: DividerOrientation,
    pub inset: f32,
}

impl Default for DividerProps {
    fn default() -> Self {
        Self {
            orientation: DividerOrientation::Horizontal,
            inset: 0.0,
        }
    }
}
