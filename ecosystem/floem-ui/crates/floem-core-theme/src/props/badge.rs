/// Badge has no callbacks, so it can derive `Clone`/`Debug`.
#[derive(Debug, Clone)]
pub struct BadgeProps {
    /// Text content shown inside the badge. Ignored when `dot` is true.
    pub content: String,
    /// When true, render as a small dot with no text.
    pub dot: bool,
}
