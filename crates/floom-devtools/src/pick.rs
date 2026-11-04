//! The pick: click the app, inspect what you clicked.
//!
//! Chrome DevTools' signature interaction is the inspect button: you press it,
//! move over the page, and whatever is under the cursor gets a box drawn around
//! it and becomes the selection. This is that, for a Floom tree.
//!
//! ## Why the pick lives outside the tree
//!
//! While picking, a pointer event must *not* reach the app: clicking a button to
//! inspect it must not also press it. So the picker sits in front of the tree —
//! the caller asks it first (the way [`crate::panel::DevtoolsPanel::handle_key`]
//! is asked first for a key) and a consumed event is dropped.
//!
//! ## What the host supplies
//!
//! Two things, both already there:
//!
//! * [`floom_app::WidgetHost::node_at_point`] — the same resolution a click
//!   gets, overlays first, so a pick over an open dialog picks the dialog.
//! * [`floom_app::WidgetHost::bounds`] — the node's box, for the highlight.
//!
//! ## What the panel draws
//!
//! A highlight is a widget the host mounts as an overlay *above* the panel, so
//! the box is visible over everything including the devtools itself. It repaints
//! when the hovered node changes, and it draws nothing when nothing is hovered.

#![forbid(unsafe_code)]

use floom_core::{
    Color, CreateContext, ElementId, LayoutRequest, PaintContext, Point, PointerEvent,
    PointerPhase, Rect, UpdateContext, Widget,
};
use floom_types::Brush;

/// The highlight box's outline: Chrome's blue, at full strength.
const HIGHLIGHT_COLOR: Color = Color::rgb(0x6E, 0x9E, 0xFF);

/// The fill under the outline, so a box over a dark element still reads.
const HIGHLIGHT_FILL: u8 = 26;

/// The outline's width, in logical pixels.
const HIGHLIGHT_WIDTH: f64 = 1.5;

/// The shortcut that arms the picker, as a predicate over a key event.
///
/// `Ctrl+Shift+C`, and `Cmd+Shift+C` on macOS — Chrome's inspect-element
/// binding, and the one the panel's own documentation names. A predicate rather
/// than a stored key, for the same reason
/// [`DevtoolsShortcut`](crate::panel::DevtoolsShortcut) is one.
pub type PickShortcut = std::sync::Arc<dyn Fn(&floom_types::KeyEvent) -> bool + Send + Sync>;

/// The default pick shortcut: `Ctrl+Shift+C`, or `Cmd+Shift+C` on macOS.
///
/// `Char('c')` rather than `Char('C')` because the platform layer lowercases the
/// character it maps; shift is already carried by `modifiers`.
pub fn default_pick_shortcut() -> PickShortcut {
    use floom_types::{Key, KeyPhase, Modifiers};
    std::sync::Arc::new(|event: &floom_types::KeyEvent| {
        event.phase == KeyPhase::Down
            && event.key == Key::Char('c')
            && event.modifiers.contains(Modifiers::SHIFT)
            && (event.modifiers.contains(Modifiers::CONTROL)
                || event.modifiers.contains(Modifiers::META))
    })
}

/// What the picker is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PickPhase {
    /// Not picking: pointer events belong to the app.
    #[default]
    Idle,
    /// Picking: the next click selects and stops.
    Armed,
    /// Picking, and the pointer is moving: the hovered node is highlighted.
    Hovering,
}

/// The pick state — shared between the app's event loop and the panel.
///
/// The same mutex-guarded-cell shape [`crate::panel::DevtoolsState`] uses, and
/// for the same reason: the arm comes from a keystroke in the platform loop,
/// which is outside the tree and therefore outside the signal system.
pub struct PickState {
    phase: PickPhase,
    /// The node the pointer is over, as a live-tree key.
    hovered: Option<String>,
    /// The node a click committed to.
    picked: Option<String>,
    /// Whether the panel has already been told about `picked`.
    ///
    /// A pick outlives the click that made it — it is what the highlight keeps
    /// sitting on — so `picked` cannot be cleared on commit. But *writing* it
    /// into the panel is a one-shot: a caller that commits on every pointer
    /// event (the facade does, because it has no cheaper way to know a click
    /// happened) would otherwise re-select the last picked element on every
    /// later click, overwriting whatever the user selected since.
    committed: bool,
}

impl PickState {
    pub fn new() -> Self {
        Self {
            phase: PickPhase::Idle,
            hovered: None,
            picked: None,
            committed: false,
        }
    }

    /// Arms the picker: the next click selects, the way Chrome's inspect button
    /// does. Also what the keyboard shortcut does.
    pub fn arm(&mut self) {
        self.phase = PickPhase::Armed;
    }

    /// Disarms it, leaving any selection in place.
    pub fn disarm(&mut self) {
        self.phase = PickPhase::Idle;
        self.hovered = None;
    }

    /// Whether the picker is waiting for a click.
    pub fn armed(&self) -> bool {
        self.phase != PickPhase::Idle
    }

    /// What the picker is doing.
    pub fn phase(&self) -> PickPhase {
        self.phase
    }

    /// The key of the node under the pointer, while picking.
    pub fn hovered(&self) -> Option<&str> {
        self.hovered.as_deref()
    }

    /// The key of the node a click committed to.
    pub fn picked(&self) -> Option<&str> {
        self.picked.as_deref()
    }

    /// Records the node under the pointer.
    pub fn set_hovered(&mut self, key: Option<String>) {
        self.hovered = key;
        if self.phase == PickPhase::Armed && self.hovered.is_some() {
            self.phase = PickPhase::Hovering;
        }
    }

    /// Commits a selection and stops picking.
    pub fn set_picked(&mut self, key: String) {
        self.picked = Some(key);
        self.phase = PickPhase::Idle;
        self.hovered = None;
        // A new pick has not been shown to the panel yet.
        self.committed = false;
    }

    /// Takes the pick if the panel has not been told about it, marking it told.
    ///
    /// The one-shot read: whoever writes the selection into the panel calls
    /// this, so a second caller in the same pick gets `None` rather than
    /// re-selecting the same element.
    pub fn take_uncommitted(&mut self) -> Option<String> {
        if self.committed {
            return None;
        }
        let key = self.picked.clone()?;
        self.committed = true;
        Some(key)
    }

    /// Whether the panel has already been told about the current pick.
    pub fn committed(&self) -> bool {
        self.committed
    }

    /// Clears the selection without touching the picker.
    pub fn clear_picked(&mut self) {
        self.picked = None;
        self.committed = false;
    }
}

impl Default for PickState {
    fn default() -> Self {
        Self::new()
    }
}

/// The box drawn around the hovered or picked node.
///
/// A widget rather than a paint-time decoration because it has to sit above the
/// panel: the panel is itself an overlay, and a highlight drawn inside it would
/// be hidden whenever the pointer moved over the devtools. The host mounts this
/// as one more overlay, on top.
pub struct HighlightBox {
    /// The box to draw, in viewport coordinates, or nothing to draw.
    rect: Option<Rect>,
    /// Whether the box follows a hover (in progress) or a committed pick.
    live: bool,
}

impl HighlightBox {
    /// A highlight around `rect`, or an inert one when `None`.
    pub fn new(rect: Option<Rect>, live: bool) -> Self {
        Self { rect, live }
    }

    /// The in-place form, for a box the host already mounted.
    pub fn set_rect(&mut self, rect: Option<Rect>, live: bool) {
        self.rect = rect;
        self.live = live;
    }

    /// The box being drawn, or `None` when the highlight is inert.
    pub fn rect(&self) -> Option<Rect> {
        self.rect
    }

    /// Whether the box is following a hover rather than sitting on a pick.
    pub fn is_live(&self) -> bool {
        self.live
    }
}

impl Widget for HighlightBox {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}

    fn layout_request(&self) -> Option<LayoutRequest> {
        // The box covers the window so the highlight can be anywhere in it; the
        // paint only draws the rect that was handed over.
        Some(LayoutRequest::new().fill())
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let Some(rect) = self.rect else {
            return;
        };
        if rect.is_empty() {
            return;
        }
        cx.fill_rect_brush(
            rect,
            &Brush::from(Color::rgba(
                HIGHLIGHT_COLOR.r,
                HIGHLIGHT_COLOR.g,
                HIGHLIGHT_COLOR.b,
                HIGHLIGHT_FILL,
            )),
        );
        // The outline: four thin rects, because the paint contract has a
        // stroked-rect op and a widget should not have to fake one.
        let w = HIGHLIGHT_WIDTH;
        let top = Rect::from_origin_size(
            floom_types::Point::new(rect.x0, rect.y0),
            floom_types::Size::new(rect.width(), w),
        );
        let bottom = Rect::from_origin_size(
            floom_types::Point::new(rect.x0, rect.y1 - w),
            floom_types::Size::new(rect.width(), w),
        );
        let left = Rect::from_origin_size(
            floom_types::Point::new(rect.x0, rect.y0),
            floom_types::Size::new(w, rect.height()),
        );
        let right = Rect::from_origin_size(
            floom_types::Point::new(rect.x1 - w, rect.y0),
            floom_types::Size::new(w, rect.height()),
        );
        for edge in [top, bottom, left, right] {
            cx.fill_rect(edge, HIGHLIGHT_COLOR);
        }
    }
}

impl floom_core::WidgetAny for HighlightBox {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// A highlight is decoration over the app, not content: assistive
    /// technology should read the element it surrounds, not the box.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

/// Handles a pointer event for the picker.
///
/// Returns whether the event was consumed. A consumed event is dropped by the
/// caller, so clicking a button to inspect it does not also press it.
///
/// `key_of` maps the node the pointer is over to the live-tree key the panel
/// selects; the host owns that mapping, because it owns the tree.
pub fn pick_pointer<F>(state: &std::sync::Mutex<PickState>, event: &PointerEvent, key_of: F) -> bool
where
    F: FnOnce(Point) -> Option<String>,
{
    let Ok(mut guard) = state.lock() else {
        return false;
    };
    if !guard.armed() {
        return false;
    }
    match event.phase {
        PointerPhase::Move => {
            guard.set_hovered(key_of(event.position));
            true
        }
        PointerPhase::Down | PointerPhase::Up => {
            if event.phase == PointerPhase::Down
                && let Some(key) = key_of(event.position) {
                    guard.set_picked(key);
                }
            true
        }
        _ => false,
    }
}

/// The pointer events a picker consumes, named so a caller can log them.
pub fn is_pick_phase(phase: PointerPhase) -> bool {
    matches!(
        phase,
        PointerPhase::Move | PointerPhase::Down | PointerPhase::Up
    )
}

/// The context a picker needs from an event, named so a caller can build one.
pub struct PickEvent {
    /// Where the pointer was, in viewport coordinates.
    pub position: Point,
    /// What the pointer did.
    pub phase: PointerPhase,
}

impl From<&PointerEvent> for PickEvent {
    fn from(event: &PointerEvent) -> Self {
        Self {
            position: event.position,
            phase: event.phase,
        }
    }
}

/// The box the highlight should draw for a picked key, if the host can find it.
///
/// `bounds_of` maps a live-tree key to the node's box; the host owns that
/// mapping, because it owns the layout.
pub fn highlight_rect<F>(state: &std::sync::Mutex<PickState>, bounds_of: F) -> Option<Rect>
where
    F: FnOnce(&str) -> Option<Rect>,
{
    let guard = state.lock().ok()?;
    // A hover wins while picking: the box should follow the pointer, and the
    // committed pick is what it falls back to once picking stops.
    let key = match guard.phase() {
        PickPhase::Hovering => guard.hovered().or_else(|| guard.picked())?,
        _ => guard.picked()?,
    };
    bounds_of(key)
}

/// Whether the highlight is following a hover rather than sitting on a pick.
pub fn highlight_is_live(state: &std::sync::Mutex<PickState>) -> bool {
    state
        .lock()
        .map(|guard| guard.phase() == PickPhase::Hovering)
        .unwrap_or(false)
}
