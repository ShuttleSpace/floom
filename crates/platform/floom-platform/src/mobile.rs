// SPDX-License-Identifier: MIT
//! Platform-neutral mobile window-level facts and pure logic.
//!
//! The concrete backends (`floom-platform-android` / `floom-platform-ios`) own
//! the OS calls; this module holds the types and pure transformations they and
//! their callers share. Keeping it OS-free means it is host-testable and adds
//! no dependency to the core ABI.

use floom_types::{Color, Insets};

pub mod android_table;

/// Stable soft-keyboard state reported to the app.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SoftKeyboardState {
    /// Whether the soft keyboard is currently shown.
    pub visible: bool,
    /// Visible keyboard height in logical pixels (0 when not visible).
    pub height: f64,
}

impl SoftKeyboardState {
    /// The screen region the keyboard covers, as bottom insets.
    pub fn insets(&self) -> Insets {
        if self.visible && self.height > 0.0 {
            Insets {
                top: 0.0,
                right: 0.0,
                bottom: self.height,
                left: 0.0,
            }
        } else {
            Insets::ZERO
        }
    }
}

/// Hysteresis filter that collapses small / transient keyboard-height reports
/// into a stable value, preventing layout jitter during the show/hide
/// animation.
pub struct KeyboardHeightFilter {
    threshold: f64,
    current: f64,
}

impl KeyboardHeightFilter {
    /// A filter with the given hysteresis threshold (logical px).
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            current: 0.0,
        }
    }

    /// A filter with the default 8px threshold.
    pub fn with_default_threshold() -> Self {
        Self::new(8.0)
    }

    /// Current stable height.
    pub fn current(&self) -> f64 {
        self.current
    }

    /// Feed a raw reported height. Returns `Some(new_height)` when the stable
    /// height should update, otherwise `None`.
    pub fn push(&mut self, reported: f64) -> Option<f64> {
        let reported = reported.max(0.0);
        let visibility_changed = (self.current == 0.0) != (reported == 0.0);
        let moved = (reported - self.current).abs() >= self.threshold;
        if visibility_changed || moved {
            self.current = reported;
            Some(self.current)
        } else {
            None
        }
    }
}

/// Text the IME handed to the platform since the last read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImeInput {
    /// Finished text. Append it.
    Commit(String),
    /// The composition in progress. Replaces any previous preedit; empty clears it.
    Preedit(String),
}

/// Split a field's raw contents into `(committed, composing)`.
///
/// `start`/`end` are UTF-16 code units (what Android's composing span reports);
/// Rust strings are UTF-8, so this converts rather than slicing directly.
pub fn split_composition(text: &str, start: i32, end: i32) -> (String, String) {
    if start < 0 || end < start {
        return (text.to_string(), String::new());
    }
    let (Some(start), Some(end)) = (
        utf16_offset_to_byte(text, start as usize),
        utf16_offset_to_byte(text, end as usize),
    ) else {
        return (text.to_string(), String::new());
    };

    let mut committed = String::with_capacity(text.len() - (end - start));
    committed.push_str(&text[..start]);
    committed.push_str(&text[end..]);
    (committed, text[start..end].to_string())
}

/// The part of `committed` that was not there before, or `None` when the text
/// was rewritten rather than appended.
pub fn newly_committed<'a>(previous: &str, committed: &'a str) -> Option<&'a str> {
    committed.strip_prefix(previous)
}

/// Byte offset of the `utf16`-th UTF-16 code unit, or `None` past the end / on
/// a surrogate-pair boundary.
fn utf16_offset_to_byte(text: &str, utf16: usize) -> Option<usize> {
    let mut units = 0usize;
    for (byte, ch) in text.char_indices() {
        if units == utf16 {
            return Some(byte);
        }
        units += ch.len_utf16();
        if units > utf16 {
            return None;
        }
    }
    (units == utf16).then_some(text.len())
}

/// Brightness of the icons drawn inside a system bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarIconBrightness {
    /// Dark icons — use on a light bar background.
    Dark,
    /// Light icons — use on a dark bar background.
    #[default]
    Light,
}

/// Describes the appearance of the OS system bars.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SystemChromeStyle {
    /// Status bar background color (`None` = default).
    pub status_bar_color: Option<Color>,
    /// Navigation bar background color (`None` = default).
    pub navigation_bar_color: Option<Color>,
    /// Icon brightness for the status bar.
    pub status_bar_icon_brightness: BarIconBrightness,
    /// Icon brightness for the navigation bar.
    pub navigation_bar_icon_brightness: BarIconBrightness,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_hysteresis() {
        let mut f = KeyboardHeightFilter::with_default_threshold();
        assert_eq!(f.push(300.0), Some(300.0));
        assert_eq!(f.push(305.0), None);
        assert_eq!(f.push(0.0), Some(0.0));
    }

    #[test]
    fn visible_state_insets_bottom_only() {
        let s = SoftKeyboardState {
            visible: true,
            height: 250.0,
        };
        assert_eq!(s.insets().bottom, 250.0);
        assert_eq!(s.insets().top, 0.0);
    }

    #[test]
    fn cjk_composition_is_split_by_utf16_units() {
        let (committed, preedit) = split_composition("你好吗", 2, 3);
        assert_eq!(committed, "你好");
        assert_eq!(preedit, "吗");
    }

    #[test]
    fn surrogate_boundary_degrades_to_all_committed() {
        assert_eq!(
            split_composition("a🦀", 1, 2),
            ("a🦀".into(), String::new())
        );
    }

    #[test]
    fn only_appended_commit_counts_as_new() {
        assert_eq!(newly_committed("hello", "hello world"), Some(" world"));
        assert_eq!(newly_committed("hello", "hell"), None);
    }
}
