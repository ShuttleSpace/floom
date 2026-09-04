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
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StyleProps {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub font_size: Option<f32>,
    pub opacity: Option<f32>,
    pub padding: Option<f32>,
}

impl StyleProps {
    pub fn merge_over(&mut self, other: &Self) {
        if other.background.is_some() {
            self.background = other.background;
        }
        if other.foreground.is_some() {
            self.foreground = other.foreground;
        }
        if other.font_size.is_some() {
            self.font_size = other.font_size;
        }
        if other.opacity.is_some() {
            self.opacity = other.opacity;
        }
        if other.padding.is_some() {
            self.padding = other.padding;
        }
    }

    /// Only typography props inherit from parent.
    pub fn inherit_typography(&mut self, parent: &Self) {
        if self.font_size.is_none() {
            self.font_size = parent.font_size;
        }
        if self.foreground.is_none() {
            self.foreground = parent.foreground;
        }
    }
}
