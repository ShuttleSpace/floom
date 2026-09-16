//! Core Widget / Element contracts (renderer- and platform-free).

#![forbid(unsafe_code)]

mod compose;
mod context;
mod element;
mod native;
mod overlay;
mod semantics;
mod widget;
mod widget_any;

pub use compose::{Align, Axis, Child, ChildBuilder, ChildKey, Inset, LayoutRequest, Position};
pub use context::{CreateContext, EventContext, PaintContext, SemanticsContext, UpdateContext};
pub use element::{Element, ElementKind};
pub use floem_types::{
    Color, ElementId, Event, ImeUpdate, InputMode, Key, KeyEvent, KeyPhase, Modifiers, NamedKey,
    NativeCompositionMode, NativeViewKind, OverlayHandle, OverlayKind, Point, PointerButton,
    PointerEvent, PointerKind, PointerPhase, Rect, ResourceId, ScrollEvent, Size,
};
pub use native::{NativeGrant, NativeRequest};
pub use overlay::OverlayRequest;
pub use semantics::{SemanticsHint, SemanticsRole, SemanticsState};
pub use widget::Widget;
pub use widget_any::WidgetAny;
