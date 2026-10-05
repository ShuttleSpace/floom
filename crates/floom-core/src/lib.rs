//! Core Widget / Element contracts (renderer- and platform-free).

#![forbid(unsafe_code)]

mod compose;
mod context;
mod element;
mod native;
mod overlay;
mod repaint;
mod semantics;
mod widget;
mod widget_any;

pub use compose::{Align, Axis, Child, ChildBuilder, ChildKey, Inset, LayoutRequest, Position};
pub use context::{CreateContext, EventContext, PaintContext, SemanticsContext, UpdateContext};
pub use element::{Element, ElementKind};
pub use floom_types::{
    AnimationSpec, Brush, Color, CompositorProp, Easing, ElementId, Event, Filter, FontWeight,
    ImeUpdate, InputMode, Insets, Key, KeyEvent, KeyPhase, Length, Modifiers, NamedKey,
    NativeCompositionMode, NativeSnapshot, NativeViewKind, OverlayHandle, OverlayKind, Point,
    PointerButton, PointerEvent, PointerKind, PointerPhase, PresenceChannel, PresenceTransition,
    Rect, RepeatMode, RepeatSpec, ResourceId, ScrollEvent, Shadow, Size, Transition,
};
pub use native::{NativeGrant, NativeRequest};
pub use overlay::OverlayRequest;
pub use repaint::{has_repaint_requests, request_repaint, take_repaint_requests};
pub use semantics::{SemanticsHint, SemanticsRole, SemanticsState};
pub use widget::Widget;
pub use widget_any::WidgetAny;
