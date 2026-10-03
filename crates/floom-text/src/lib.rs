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
pub use measure::measure_text;
