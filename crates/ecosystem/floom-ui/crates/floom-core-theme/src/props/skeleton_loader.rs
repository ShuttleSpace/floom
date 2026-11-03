/// Visual shape of a skeleton placeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkeletonType {
    /// A stack of text lines.
    Text,
    /// A single circle (e.g. an avatar/icon placeholder).
    Circle,
    /// A single rounded rectangle.
    Rect,
    /// A circle plus a couple of text lines (avatar + name).
    Avatar,
    /// A large rounded card block.
    Card,
}

/// Neutral props shared by every design system's `SkeletonLoader`.
#[derive(Debug, Clone)]
pub struct SkeletonLoaderProps {
    pub skeleton_type: SkeletonType,
    /// Explicit width in dp (overrides the type default).
    pub width: Option<f32>,
    /// Explicit height in dp (overrides the type default).
    pub height: Option<f32>,
    /// Number of text lines when `skeleton_type` is `Text` or `Avatar`.
    pub lines: usize,
    /// Whether the placeholder should shimmer (best-effort; static fallback
    /// when the current Floom build has no animation frame hook).
    pub animated: bool,
}

impl Default for SkeletonLoaderProps {
    fn default() -> Self {
        Self {
            skeleton_type: SkeletonType::Text,
            width: None,
            height: None,
            lines: 3,
            animated: true,
        }
    }
}
