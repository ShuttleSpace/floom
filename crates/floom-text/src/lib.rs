//! Renderer-neutral text layout for Floom 2.0.
//!
//! Shaping / bidi / line-breaking live here (Parley). Renderers consume
//! [`GlyphRun`] / [`LaidOutText`] only — they must not own shaping.

mod cache;
mod layout;
mod measure;

pub use cache::{DEFAULT_LAYOUT_CACHE_CAPACITY, LayoutCache};
pub use layout::{
    Glyph, GlyphRun, LaidOutText, LineMetrics, TextAlign, TextLayoutOptions, TextShaper,
};
pub use measure::{measure, measure_text, measure_weighted_text};

/// Re-exported because [`TextLayoutOptions::weight`] and
/// [`measure_weighted_text`] are public API: a caller naming either needs the
/// type, and should not have to add a dependency on `floom-types` to do it.
pub use floom_types::FontWeight;
