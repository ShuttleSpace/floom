//! The Floom 2.0 inspection protocol (Phase 23.1).
//!
//! Everything a devtools frontend and a headless backend share, and nothing
//! else: versioned, serde-serializable snapshot types for the element tree,
//! resolved styles, layout requests, the painted scene, and performance
//! counters. This crate deliberately links no runtime — a frontend can parse a
//! snapshot without a window, and a backend can serve one without a GUI.
//!
//! Versioning has concrete semantics, mirroring [`floom_ui_ir`]'s rule and for
//! the same reason: a reader must never be handed fields it cannot represent.
//! A snapshot with the same major and an **older or equal minor** loads; a
//! newer minor is refused (the reader cannot faithfully round-trip unknown
//! fields); a different major is refused outright.

#![forbid(unsafe_code)]

mod error;
mod json;
mod layout;
mod perf;
mod scene;
mod style;
mod tree;
mod version;

pub use error::ProtocolError;
pub use json::JsonSnapshot;
pub use layout::{LayoutDto, LayoutEntry, LayoutSnapshot};
pub use perf::{AllocSummary, CacheStats, FrameSummary, PerformanceSnapshot, StageSpan};
pub use scene::SceneLines;
pub use style::{STYLE_STATES, StyleEntry, StylePropsDto, StyleSnapshot, is_known_style_state};
pub use tree::{TreeNodeInfo, TreeSnapshot};
pub use version::ProtocolVersion;

/// A colour on the wire. The runtime's `Color` is not serde-aware, so the
/// protocol carries its own component-wise copy; the inspector converts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

impl From<floom_types::Color> for Rgba {
    fn from(c: floom_types::Color) -> Self {
        Self::new(c.r, c.g, c.b, c.a)
    }
}

impl From<Rgba> for floom_types::Color {
    fn from(c: Rgba) -> Self {
        floom_types::Color::rgba(c.r, c.g, c.b, c.a)
    }
}

/// Where a node came from. The host fills this in (from a DSL span, a proc
/// macro, or a hand-written table); the protocol only carries it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: u32,
}

impl std::fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}
