//! Core Widget / Element contracts (renderer- and platform-free).

#![forbid(unsafe_code)]

mod compose;
mod context;
mod element;
mod semantics;
mod widget;
mod widget_any;

pub use compose::{Align, Axis, Child, ChildBuilder, ChildKey, Inset, LayoutRequest, Position};
pub use context::{CreateContext, EventContext, PaintContext, SemanticsContext, UpdateContext};
pub use element::{Element, ElementKind};
pub use floem_types::{
    Color, ElementId, Event, Key, KeyEvent, KeyPhase, Modifiers, NamedKey, Point, PointerButton,
    PointerEvent, PointerKind, PointerPhase, Rect, ResourceId, ScrollEvent, Size,
};
pub use semantics::{SemanticsHint, SemanticsRole, SemanticsState};
pub use widget::Widget;
pub use widget_any::WidgetAny;
