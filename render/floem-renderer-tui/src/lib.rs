//! Terminal Scene renderer for Floem 2.0.
//!
//! A terminal is not a pixel grid — it is a **character grid**. This crate
//! therefore does not downsample a rasterised picture into coloured blocks (the
//! approach image viewers use, and the one that renders every label as mush at
//! ordinary terminal sizes). It paints the scene into characters: a `Rect`
//! becomes a run of background cells, and a `Text` node becomes the actual
//! characters it was shaped from, written at the cell its glyph occupies.
//!
//! That second half is why [`ResourceKind::GlyphRun`] retains the source string:
//! shaping is lossy (a ligature turns several characters into one glyph, and a
//! fallback font can split one cluster into several), so the characters cannot
//! be recovered from glyph ids later.
//!
//! ## What this backend deliberately does not do
//!
//! Everything a character grid cannot express — vector paths, images, blurs,
//! custom shaders, gradients — is **skipped, not approximated**. The paint pass
//! reports how much it dropped in [`PaintStats`] so a caller can see that its
//! scene contains things a terminal cannot show, rather than discovering it in
//! a screenshot. Approximating them (a gradient's first stop, say) would be a
//! silent lie about what the scene asked for.
//!
//! [`ResourceKind::GlyphRun`]: floem_scene::ResourceKind::GlyphRun

mod ansi;
mod grid;
mod presenter;

pub use ansi::{EncodeStats, encode, encode_with_stats};
pub use grid::{Cell, PaintStats, TuiGrid, paint_scene};
pub use presenter::{TerminalSize, TuiPresenter, frame_info};
