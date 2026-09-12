//! Core Widget / Element contracts (renderer- and platform-free).

#![forbid(unsafe_code)]

mod context;
mod element;
mod widget;

pub use context::{
    CreateContext, EventContext, PaintContext, SemanticsContext, UpdateContext,
};
pub use element::{Element, ElementKind};
pub use floem_types::{
    Color, ElementId, Event, Key, KeyEvent, KeyPhase, Modifiers, NamedKey, Point, PointerButton,
    PointerEvent, PointerKind, PointerPhase, Rect, ResourceId, ScrollEvent, Size,
};
pub use widget::Widget;
