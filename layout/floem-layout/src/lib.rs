//! Backend-neutral layout tree. Concrete engine IDs never appear in the public API.

mod taffy_adapter;

pub use taffy_adapter::{
    AbsoluteInset, AlignItems, FlexDirection, JustifyContent, LayoutEngine, LayoutNode,
    LayoutOutput, LayoutStyle, MeasureFn, Position,
};
