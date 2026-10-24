//! Core Widget / Element contracts (renderer- and platform-free).

#![forbid(unsafe_code)]

mod compose;
mod context;
mod element;
mod native;
mod overlay;
mod repaint;
mod scroll;
mod semantics;
mod widget;
mod widget_any;

/// One node of a host's tree, as plain data.
///
/// The shape an inspector describes into its own: the host names the node, says
/// what tier it is, and says which node it hangs from. Nothing here is a type
/// from a higher layer, which is the whole point — a host at L7 can hand these
/// over without depending on the L8 tooling that reads them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostedNode {
    /// The host's own name for the node.
    pub key: String,
    /// What tier the node mounts as.
    pub kind: HostedKind,
    /// The host's own element id for this node.
    ///
    /// Present so a child can name its parent: the link is the host's, and the
    /// host's ids are the only thing both sides can agree on. Without it a
    /// parent reference would have to be resolved by key, which is a string
    /// comparison per edge.
    pub element: ElementId,
    /// The element id of the node's parent, or `None` for the root.
    pub parent: Option<ElementId>,
}

/// The three tiers the inspector reports, in the host's own vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostedKind {
    /// A container: it has children.
    Host,
    /// A leaf widget.
    Primitive,
    /// A native composition.
    Composition,
}

pub use compose::{Align, Axis, Child, ChildBuilder, ChildKey, Inset, LayoutRequest, Position};
pub use context::{CreateContext, EventContext, PaintContext, SemanticsContext, UpdateContext};
pub use element::{Element, ElementKind};
pub use floom_types::{
    AnimationSpec, Brush, Color, CompositorProp, Easing, Edges, ElementId, Event, Filter,
    FontWeight, ImeUpdate, InputMode, Insets, Key, KeyEvent, KeyPhase, Length, Modifiers, NamedKey,
    NativeCompositionMode, NativeSnapshot, NativeViewKind, OverlayHandle, OverlayKind, Point,
    PointerButton, PointerEvent, PointerKind, PointerPhase, PresenceChannel, PresenceTransition,
    Rect, RepeatMode, RepeatSpec, ResourceId, ScrollEvent, Shadow, Size, Transition,
};
pub use native::{NativeGrant, NativeRequest};
pub use overlay::OverlayRequest;
pub use repaint::{has_repaint_requests, request_repaint, take_repaint_requests};
pub use scroll::{ScrollOffset, clamp_offset, max_offset};
pub use semantics::{SemanticsHint, SemanticsRole, SemanticsState};
pub use widget::Widget;
pub use widget_any::WidgetAny;
