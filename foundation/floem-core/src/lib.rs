//! Core Widget / Element contracts (renderer- and platform-free).

#![forbid(unsafe_code)]

mod compose;
mod context;
mod element;
mod overlay;
mod semantics;
mod widget;
mod widget_any;

pub use compose::{Align, Axis, Child, ChildBuilder, ChildKey, Inset, LayoutRequest, Position};
pub use context::{CreateContext, EventContext, PaintContext, SemanticsContext, UpdateContext};
pub use element::{Element, ElementKind};
pub use floem_types::{
    Color, ElementId, Event, ImeUpdate, Key, KeyEvent, KeyPhase, Modifiers, NamedKey,
    OverlayHandle, OverlayKind, Point, PointerButton, PointerEvent, PointerKind, PointerPhase,
    Rect, ResourceId, ScrollEvent, Size,
};
pub use overlay::OverlayRequest;
pub use semantics::{SemanticsHint, SemanticsRole, SemanticsState};
pub use widget::Widget;
pub use widget_any::WidgetAny;
