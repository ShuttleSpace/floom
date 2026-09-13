//! Convenience intrinsic-size measurement for widget layout.
//!
//! Widgets report their intrinsic size through `Widget::layout_request`, but a
//! widget must not own a [`TextShaper`]: font contexts are expensive and
//! widgets are rebuilt freely. This module hangs one shaper off each thread
//! (text work is main-thread by construction) and exposes a one-call measure.

use std::cell::RefCell;

use floem_types::Size;

use crate::layout::{TextAlign, TextLayoutOptions, TextShaper};

thread_local! {
    static SHAPER: RefCell<TextShaper> = RefCell::new(TextShaper::new());
}

/// Measures `text` laid out with the default font stack at `font_size`.
///
/// `max_width` bounds line breaking; pass [`f64::INFINITY`] for the intrinsic
/// (single-line-per-`\n`) size. The result is the laid-out extent: the widest
/// line and the total line height.
///
/// Empty text measures to zero: an empty [`crate::GlyphRun`] list carries no
/// baseline metrics, and a zero-size box is exactly what layout should give an
/// empty label.
pub fn measure_text(text: &str, font_size: f32, max_width: f64) -> Size {
    if text.is_empty() || font_size <= 0.0 {
        return Size::ZERO;
    }
    let options = TextLayoutOptions {
        max_width,
        font_size,
        align: TextAlign::Start,
    };
    SHAPER.with(|shaper| shaper.borrow_mut().shape(text, &options).size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_measures_to_zero() {
        assert_eq!(measure_text("", 16.0, f64::INFINITY), Size::ZERO);
    }

    #[test]
    fn nonpositive_font_size_measures_to_zero() {
        assert_eq!(measure_text("hi", 0.0, f64::INFINITY), Size::ZERO);
        assert_eq!(measure_text("hi", -3.0, f64::INFINITY), Size::ZERO);
    }

    #[test]
    fn width_scales_with_font_size() {
        let small = measure_text("Hello world", 12.0, f64::INFINITY);
        let large = measure_text("Hello world", 24.0, f64::INFINITY);
        assert!(small.width > 0.0);
        assert!(large.width > small.width, "bigger size, wider line");
        assert!(large.height > small.height);
    }

    #[test]
    fn explicit_newlines_break_lines() {
        let single = measure_text("ab", 16.0, f64::INFINITY);
        let wrapped = measure_text("a\nb", 16.0, f64::INFINITY);
        assert!(wrapped.height > single.height, "two lines, taller box");
        assert!(wrapped.width < single.width, "each line is narrower");
    }

    #[test]
    fn max_width_wraps_and_grows_height() {
        let free = measure_text(
            "the quick brown fox jumps over the lazy dog",
            16.0,
            f64::INFINITY,
        );
        let constrained = measure_text("the quick brown fox jumps over the lazy dog", 16.0, 120.0);
        assert!(constrained.width <= 120.0 + f64::EPSILON);
        assert!(constrained.height > free.height, "wrapping adds lines");
    }

    #[test]
    fn repeated_calls_reuse_the_thread_shaper() {
        // No observable state, but exercises the thread-local borrow twice and
        // guards against re-entrancy panics (RefCell double-borrow).
        let a = measure_text("measure", 14.0, f64::INFINITY);
        let b = measure_text("measure", 14.0, f64::INFINITY);
        assert_eq!(a, b, "same input, same measured extent");
    }
}
