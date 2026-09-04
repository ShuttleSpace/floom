//! Renderer-neutral text layout for Floem 2.0.
//!
//! Shaping / bidi / line-breaking live here (Parley). Renderers consume
//! [`GlyphRun`] / [`LaidOutText`] only — they must not own shaping.

mod cache;
mod layout;

pub use cache::LayoutCache;
pub use layout::{
    Glyph, GlyphRun, LaidOutText, LineMetrics, TextAlign, TextLayoutOptions, TextShaper,
};
