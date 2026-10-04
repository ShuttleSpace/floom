//! Backend-neutral layout tree. Concrete engine IDs never appear in the public API.

mod taffy_adapter;

/// Re-exported because [`LayoutStyle`]'s dimensions are `Option<Length>`: a
/// caller writing one needs the type, and should not have to know which
/// lower crate it lives in.
pub use floom_types::Length;

pub use taffy_adapter::{
    AbsoluteInset, AlignItems, FlexDirection, JustifyContent, LayoutEngine, LayoutNode,
    LayoutOutput, LayoutStyle, MeasureFn, Position,
};
