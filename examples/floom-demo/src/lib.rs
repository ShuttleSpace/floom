// SPDX-License-Identifier: MIT
//! The Floom component gallery.
//!
//! A gallery of every component the `floom-ui` satellite provides, shown in
//! all three of its design systems side by side.
//!
//! - [`ds`] holds the per-design-system component types, generated from the
//!   satellite by `tools/gen.py`.
//! - [`props_sources`] holds the library's own props sources, which the
//!   gallery parses at run time so a props table cannot drift from the struct
//!   it describes.
//!
//! Run it with `cargo run -p floom-demo`.

pub mod api;
pub mod ds;
pub mod pages;
pub mod props_sources;
pub mod registry;
