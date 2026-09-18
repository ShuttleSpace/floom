#[derive(Debug, Clone, Default)]
pub struct ButtonGroupProps {
    /// When true, buttons are joined with no gap and shared borders (segmented control).
    pub joined: bool,
    /// When true, lay the buttons out vertically instead of horizontally.
    pub vertical: bool,
    /// Gap between buttons when not joined.
    pub gap: f32,
}
