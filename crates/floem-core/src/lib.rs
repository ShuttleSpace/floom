//! Core Widget / Element contracts (renderer- and platform-free).

#![forbid(unsafe_code)]

mod context;
mod element;
mod widget;

pub use context::{
    CreateContext, EventContext, PaintContext, SemanticsContext, UpdateContext,
};
pub use element::{Element, ElementKind};
pub use floem_types::{Color, ElementId, Point, Rect, ResourceId, Size};
pub use widget::Widget;
