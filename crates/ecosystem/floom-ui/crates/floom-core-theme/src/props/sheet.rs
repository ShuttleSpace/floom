/// Edge from which a sheet slides in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetSide {
    /// Slides up from the bottom; spans the full width.
    Bottom,
    /// Slides down from the top; spans the full width.
    Top,
    /// Slides in from the start (left in LTR); full height.
    Start,
    /// Slides in from the end (right in LTR); full height.
    End,
}

/// Neutral props shared by every design system's `Sheet`.
///
/// A sheet is an overlay surface anchored to one edge of the screen. The
/// surface content is supplied via the `child(view)` builder.
#[derive(Debug, Clone)]
pub struct SheetProps {
    pub side: SheetSide,
    /// Whether the sheet is currently visible.
    pub open: bool,
    /// Apply an elevated shadow to the panel.
    pub elevated: bool,
}

impl Default for SheetProps {
    fn default() -> Self {
        Self {
            side: SheetSide::Bottom,
            open: false,
            elevated: true,
        }
    }
}
