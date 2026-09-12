//! Tier-0 primitive widgets for Floem 2.0.
//!
//! The widget set mirrors the architecture's tier list: this crate holds the
//! Tier-0 primitives ([`primitives`]); interactive controls, navigation and
//! data widgets land in sibling crates as their phases complete.
//!
//! Primitives depend only on renderer-neutral contracts (`floem-core`),
//! platform-neutral value types (`floem-types`), the retained scene
//! (`floem-scene`) and the semantics schema (`floem-accessibility`). They never
//! reach a concrete renderer, windowing system, or GPU backend.

#![forbid(unsafe_code)]

pub mod primitives;
pub mod testkit;

pub use primitives::{
    Axis, Container, Divider, Icon, Image, ImageFit, SemanticsHint, Spacer, Text,
};
