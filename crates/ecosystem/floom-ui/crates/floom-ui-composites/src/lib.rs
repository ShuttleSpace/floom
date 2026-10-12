//! Composite controls that belong to a design system rather than to the engine.
//!
//! The layering runs: `floom-widgets` holds primitives (boxes, text, icons,
//! pressable surfaces), this crate holds the assemblies a designer would
//! recognise — a tab bar, a tree, a counter — and `floom-ui`'s
//! per-design-system crates supply the tokens.
//!
//! Everything here used to live in `floom-widgets`. None of it is a primitive:
//! each one encodes a choice about what a control *is* rather than how a box
//! behaves, and the engine is the wrong layer for that. See `Cargo.toml` for
//! why the dependency cannot run the other way.
//!
//! The first to move is [`NavHost`], because it is the cleanest case: nothing
//! in `floom-widgets` referred to it, it depends on no other widget, and it had
//! one consumer outside the crate. The rest followed once the layering held.

mod any;
mod nav;

pub use nav::{NavHandle, NavHost};
