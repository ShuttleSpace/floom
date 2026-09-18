//! Style property bag and interaction states.

use bitflags::bitflags;
use floem_types::Color;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct StyleState: u8 {
        const NORMAL   = 1 << 0;
        const HOVER    = 1 << 1;
        const PRESSED  = 1 << 2;
        const FOCUSED  = 1 << 3;
        const DISABLED = 1 << 4;
        const SELECTED = 1 << 5;
    }
}

/// Sparse style properties. `None` means unset (inherit or default).
///
/// The split between paint, type and shape matters: a design system's button
/// variants differ chiefly in shape (`radius`, `border_width`, `elevation`) and
/// those must be able to change per interaction state without the state overlay
/// dropping the parts it does not mention — see `merge_over`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StyleProps {
    // --- paint ---
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub opacity: Option<f32>,
    // --- type ---
    pub font_size: Option<f32>,
    // --- box ---
    pub padding: Option<f32>,
    pub radius: Option<f32>,
    pub border_width: Option<f32>,
    pub border_color: Option<Color>,
    /// Elevation in logical pixels. Renderer-neutral: a shadow, a scrim or a
    /// tonal overlay depending on the platform, so only the magnitude is stored.
    pub elevation: Option<f32>,
}

impl StyleProps {
    pub fn merge_over(&mut self, other: &Self) {
        if other.background.is_some() {
            self.background = other.background;
        }
        if other.foreground.is_some() {
            self.foreground = other.foreground;
        }
        if other.opacity.is_some() {
            self.opacity = other.opacity;
        }
        if other.font_size.is_some() {
            self.font_size = other.font_size;
        }
        if other.padding.is_some() {
            self.padding = other.padding;
        }
        if other.radius.is_some() {
            self.radius = other.radius;
        }
        if other.border_width.is_some() {
            self.border_width = other.border_width;
        }
        if other.border_color.is_some() {
            self.border_color = other.border_color;
        }
        if other.elevation.is_some() {
            self.elevation = other.elevation;
        }
    }

    /// Only typography props inherit from parent.
    ///
    /// Deliberately excludes shape: inheriting a radius would round a child for
    /// no stated reason, and inheriting a border would draw boxes nobody asked
    /// for. Text colour travels with `font_size` because a label rendered in the
    /// parent's colour is the point of a type style.
    pub fn inherit_typography(&mut self, parent: &Self) {
        if self.font_size.is_none() {
            self.font_size = parent.font_size;
        }
        if self.foreground.is_none() {
            self.foreground = parent.foreground;
        }
    }
}
