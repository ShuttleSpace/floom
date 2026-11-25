// SPDX-License-Identifier: MIT
//! Core Widget / Element contracts (renderer- and platform-free).

#![forbid(unsafe_code)]

mod compose;
mod context;
mod element;
pub mod error;
pub mod ime;
mod native;
mod overlay;
mod paint_style;
mod rebuild;
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
///
/// The last two fields are what make the Style and Layout readouts answerable
/// at all. A host that mounts engine widgets has no document and therefore no
/// IR props to resolve; the only description of the node is the node itself.
/// So the host asks the widget what it paints ([`Widget::paint_style`]) and how
/// it wants to be laid out, and carries both across the bridge as plain data.
/// A host that cannot answer — a node backed by a widget that does not
/// implement `paint_style`, a composite with no layout request — says `None`,
/// and the readout shows an empty section rather than an invented one.
///
/// [`Widget::paint_style`]: crate::Widget::paint_style
#[derive(Debug, Clone, PartialEq)]
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
    /// What the node's widget says it paints, when it says anything.
    pub style: Option<PaintStyle>,
    /// Where the node was actually laid out, once a frame has run.
    ///
    /// `None` before the first layout: the box a widget *asks* for and the box
    /// it got are different questions, and a readout that mixed them would be
    /// wrong in the one case a readout exists for.
    pub bounds: Option<Rect>,
    /// How the node's box wants to be laid out, when it declares anything.
    ///
    /// The parent's request wins where there is one: a child's slot is sized by
    /// whoever placed it, and reporting only the widget's own preference would
    /// describe a box that was never laid out that way. `WidgetHost::tree_nodes`
    /// is where that choice is made.
    pub layout: Option<LayoutRequest>,
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
pub use error::{ErrorBoundary, Outcome, Report, Severity};
pub use floom_types::{
    AnimationSpec, Brush, Color, CompositorProp, Easing, Edges, ElementId, Event, Filter,
    FontWeight, ImeUpdate, InputMode, Insets, Key, KeyEvent, KeyPhase, Length, Modifiers, NamedKey,
    NativeCompositionMode, NativeSnapshot, NativeViewKind, OverlayHandle, OverlayKind, Point,
    PointerButton, PointerEvent, PointerKind, PointerPhase, PresenceChannel, PresenceTransition,
    Rect, RepeatMode, RepeatSpec, ResourceId, ScrollEvent, Shadow, Size, Transition,
};
pub use native::{NativeGrant, NativeRequest};
pub use overlay::OverlayRequest;
pub use paint_style::PaintStyle;
pub use rebuild::{request_rebuild_root, take_rebuild_root};
pub use repaint::{has_repaint_requests, request_repaint, take_repaint_requests};
pub use scroll::{ScrollOffset, clamp_offset, max_offset};
pub use semantics::{SemanticsHint, SemanticsRole, SemanticsState};
pub use widget::Widget;
pub use widget_any::WidgetAny;
