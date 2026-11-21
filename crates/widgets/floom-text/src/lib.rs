// SPDX-License-Identifier: MIT
//! Renderer-neutral text layout for Floom 2.0.
//!
//! Shaping / bidi / line-breaking live here (Parley). Renderers consume
//! [`GlyphRun`] / [`LaidOutText`] only — they must not own shaping.

mod cache;
mod editor_layout;
pub mod glyph;
mod layout;
mod measure;
mod system_fonts;

pub use cache::{DEFAULT_LAYOUT_CACHE_CAPACITY, LayoutCache};
pub use editor_layout::{CaretAffinity, StyledRun, TextBrush, TextLayout};
pub use layout::{
    Glyph, GlyphRun, LaidOutText, LineMetrics, TextAlign, TextLayoutOptions, TextShaper,
};
pub use measure::{
    measure, measure_text, measure_weighted_text, measure_weighted_text_in, register_font,
};

/// Re-exported because [`TextLayoutOptions::weight`], [`TextLayoutOptions::family`]
/// and [`measure_weighted_text`] are public API: a caller naming any of them
/// needs the type, and should not have to add a dependency on `floom-types` to
/// do it.
pub use floom_types::{FontFamily, FontWeight};
