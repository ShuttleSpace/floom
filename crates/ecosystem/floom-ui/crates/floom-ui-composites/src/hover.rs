//! `HoverBox` — a box that fills while the pointer is over it.
//!
//! The engine had no such thing, and three floem-ui kits each carried their
//! own copy of it: the *shape* of a hover is the same whoever draws it, and
//! the only thing that differs between design systems is the colour of the
//! fill — which is the caller's answer, not the engine's.
//!
//! ## What it is
//!
//! A box around somebody else's content, with a fill painted *under* that
//! content while the pointer rests on it. A widget paints itself before its
//! children, so the fill lands underneath, the way v1's `Container` background
//! did.
//!
//! ## The pointer event is not consumed
//!
//! v1's hover let the event through, so a click on the child still reached the
//! child. This one does the same: a hover is a reaction, not a claim.

use std::sync::Arc;

use floom_core::{
    Child, Color, CreateContext, ElementId, Event, EventContext, LayoutRequest, PaintContext,
    SemanticsContext, UpdateContext, Widget, WidgetAny,
};
use floom_types::PointerPhase;

use floom_widgets::primitives::{emit_semantics, SemanticsHint};
use floom_widgets::{read_flag, write_flag, SharedFlag};

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn(bool) + Send + Sync + 'static>;

/// A box that fills while the pointer is over it.
#[derive(Clone)]
pub struct HoverBox {
    child: Child,
    /// Whether the pointer is over the box.
    hovered: SharedFlag,
    /// The fill under the pointer.
    highlight: Color,
    on_hover_change: Option<Callback>,
    /// What the flag was when this widget last acted, so a change is a change
    /// rather than a fresh read every frame.
    last_hovered: bool,
}

impl std::fmt::Debug for HoverBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HoverBox")
            .field("hovered", &read_flag(&self.hovered))
            .finish_non_exhaustive()
    }
}

impl Default for HoverBox {
    fn default() -> Self {
        Self::new()
    }
}

impl HoverBox {
    pub fn new() -> Self {
        Self {
            child: Child::new(floom_core::ChildKey::index(0), LayoutRequest::new(), || {
                Box::new(floom_widgets::Text::new(""))
            }),
            hovered: floom_widgets::shared_flag(false),
            highlight: Color::rgba(0, 0, 0, 0),
            on_hover_change: None,
            last_hovered: false,
        }
    }

    /// The content the box wraps.
    pub fn child(mut self, child: Child) -> Self {
        self.child = child;
        self
    }

    /// The fill painted while the pointer is over the box.
    pub fn highlight(mut self, color: Color) -> Self {
        self.highlight = color;
        self
    }

    /// The flag that says whether the pointer is over the box.
    ///
    /// Shared with the caller, so a caller that wants to *read* the hover --
    /// to reveal something of its own, say -- reads the same flag the widget
    /// writes.
    pub fn hovered_flag(mut self, flag: SharedFlag) -> Self {
        self.hovered = flag;
        self
    }

    /// Called with the new hover state whenever it changes.
    pub fn on_hover_change(mut self, callback: impl Fn(bool) + Send + Sync + 'static) -> Self {
        self.on_hover_change = Some(Arc::new(callback));
        self
    }

    /// Whether the pointer is over the box.
    pub fn is_hovered(&self) -> bool {
        read_flag(&self.hovered)
    }

    /// The flag the hover state travels in.
    pub fn flag(&self) -> SharedFlag {
        self.hovered.clone()
    }

    /// Records the new state, and reports a change.
    fn set_hovered(&mut self, hovered: bool) {
        if read_flag(&self.hovered) == hovered {
            return;
        }
        write_flag(&self.hovered, hovered);
        if let Some(callback) = &self.on_hover_change {
            callback(hovered);
        }
    }
}

impl Widget for HoverBox {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let hovered = read_flag(&self.hovered);
        if hovered == self.last_hovered {
            return;
        }
        self.last_hovered = hovered;
        // The fill is this box's own decoration, so this box is what has to
        // invalidate it. A repaint and nothing else: the box does not move.
        cx.mark_dirty(id);
    }

    fn children(&self) -> Vec<Child> {
        vec![self.child.clone()]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        None
    }

    /// The fill under the pointer, under the child.
    ///
    /// A widget paints itself before its children, so the fill lands *under*
    /// the content -- the way v1's `Container` background did.
    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        if !read_flag(&self.hovered) {
            return;
        }
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        cx.fill_rect(bounds, self.highlight);
    }

    /// The pointer arriving and leaving.
    ///
    /// The event is not consumed: v1's hover let the event through, so a
    /// click on the child still reached the child.
    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        let Event::Pointer(pointer) = cx.event() else {
            return false;
        };
        match pointer.phase {
            PointerPhase::Enter | PointerPhase::Move => self.set_hovered(true),
            PointerPhase::Leave | PointerPhase::Cancel => self.set_hovered(false),
            _ => {}
        }
        false
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(SemanticsHint::decorative(), cx);
    }
}

impl WidgetAny for HoverBox {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// The box is a decoration; the content inside it names itself.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}
