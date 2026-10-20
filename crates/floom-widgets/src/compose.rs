//! Layout vocabulary — the words a screen is assembled from.
//!
//! These sit above the primitives and describe *structure*: a [`Stack`] arranges
//! children along an axis, a [`List`] mirrors a data collection with keyed rows,
//! a [`Grid`] lays cells out in columns, [`Conditional`] swaps a subtree by a
//! boolean, [`Crossfade`] and [`AnimatedContent`] animate the swap, [`Opacity`]
//! fades a subtree, and [`Scroll`] gives it a viewport that moves.
//!
//! Every composite owns only builders (`Arc<dyn Fn ..>`), never the widgets
//! behind them: [`Widget::children`] hands fresh [`Child`] slots to the host,
//! which diffs them by [`ChildKey`] and reuses mounted widgets across updates.
//! That is what lets a [`List`] keep each row's state when an item is inserted in
//! the middle.
//!
//! Composites that change their children at runtime flag the change from
//! [`Widget::update`] by calling [`UpdateContext::mark_children_dirty`]; the host
//! then reconciles just the affected subtree instead of repainting everything.
//!
//! What is *not* here any more is every widget that decides what a control looks
//! like. The overlay and its three windows -- dialog, popup, bottom sheet -- plus
//! the tab bar, the accordion and the select group moved to `floom-ui-composites`,
//! one layer up, because a modal card or a tab strip is a product decision and
//! this module is arithmetic. They compose the vocabulary below; none of it
//! refers back to them.
use std::any::Any;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use floom_accessibility::SemanticsRole;
use floom_core::{
    Align, AnimationSpec, Axis, Child, ChildKey, CompositorProp, CreateContext, Easing, ElementId,
    Event, EventContext, Inset, LayoutRequest, PaintContext, Position, PresenceTransition,
    ScrollOffset, SemanticsContext, Size, Transition, UpdateContext, Widget, WidgetAny,
};

use crate::primitives::SemanticsHint;

// ---------------------------------------------------------------------------
// Stack
// ---------------------------------------------------------------------------

/// Lays out its children along one [`Axis`] with a fixed gap.
#[derive(Clone)]
pub struct Stack {
    axis: Axis,
    request: Option<LayoutRequest>,
    gap: f32,
    main_align: Option<Align>,
    cross_align: Option<Align>,
    children: Vec<Child>,
}

impl std::fmt::Debug for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stack")
            .field("axis", &self.axis)
            .field("gap", &self.gap)
            .field("children", &self.children.len())
            .finish()
    }
}

impl Stack {
    /// A stack laid out along `axis`, no gap, default (stretch) alignment.
    pub fn new(axis: Axis) -> Self {
        Self {
            axis,
            request: None,
            gap: 0.0,
            main_align: None,
            cross_align: None,
            children: Vec::new(),
        }
    }

    /// Gap between children in logical pixels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// How many slots this stack holds.
    ///
    /// Present because [`crate::Surface`] has the same question and the same
    /// answer, and a caller building a row should not have to know which of the
    /// two it holds.
    /// The axis this stack lays its children out along.
    ///
    /// A reader for the one field that decides what a stack *is*: two `Stack`s with
    /// the same children are different components if one is a column and the other a
    /// row, and until this existed only the type name said so.
    pub fn axis(&self) -> Axis {
        self.axis
    }

    /// This stack's own layout request: its size, padding, and so on.
    ///
    /// Its siblings all have this — [`crate::Surface::request`] and
    /// [`crate::Button::layout_request`] both exist — and a stack did not, so a
    /// component whose row *is* a stack of boxes had nowhere to say how tall the row
    /// is. Found by porting the tree view, whose row is `height(row_height)` and
    /// nothing else: the obvious translation was to wrap every row in a `Surface`,
    /// which would have added a node per row to carry one number.
    pub fn request(mut self, request: LayoutRequest) -> Self {
        self.request = Some(request);
        self
    }

    /// The in-place form of [`Self::request`].
    pub fn set_request(&mut self, request: Option<LayoutRequest>) -> &mut Self {
        self.request = request;
        self
    }

    /// The stack's own request, if it declared one.
    ///
    /// Named `declared_request` rather than `layout_request` on purpose: an inherent
    /// method of that name **shadows** the `Widget` trait method, and the first
    /// version of this reader did — so a row's `gap` and its alignment silently
    /// vanished from the answer, because the stored request does not carry them and
    /// only the trait method merges the stack's own nature over it.
    ///
    /// That is the second time in this port that adding a convenient reader cost a
    /// real piece of behaviour, and both times the compiler was silent.
    pub fn declared_request(&self) -> Option<LayoutRequest> {
        self.request
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// How this stack aligns its children on the main and cross axes.
    pub fn align(mut self, main: Align, cross: Align) -> Self {
        self.main_align = Some(main);
        self.cross_align = Some(cross);
        self
    }

    /// Appends a prebuilt child slot.
    pub fn child(mut self, child: Child) -> Self {
        self.children.push(child);
        self
    }

    /// Appends a child auto-keyed by position with default layout.
    pub fn push(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        let key = ChildKey::index(self.children.len());
        self.children
            .push(Child::new(key, LayoutRequest::new(), build));
        self
    }

    /// Appends a child auto-keyed by position with an explicit layout.
    pub fn push_layout(
        mut self,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        let key = ChildKey::index(self.children.len());
        self.children.push(Child::new(key, layout, build));
        self
    }

    /// Appends a child keyed by a stable id (state follows the item, not the slot).
    pub fn push_keyed(
        mut self,
        id: u64,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.children
            .push(Child::new(ChildKey::keyed(id), LayoutRequest::new(), build));
        self
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Widget for Stack {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}

    fn children(&self) -> Vec<Child> {
        self.children.clone()
    }

    /// The stored request — a caller's size, padding, margins — with the stack's own
    /// nature laid over it.
    ///
    /// Axis, gap and alignment come from the stack rather than the caller, because
    /// they are what a `Stack` *is*: a caller who set them differently would be
    /// describing a different widget. Everything else the caller declared survives,
    /// which is what makes `Stack::new(..).request(height(..))` mean "a stack, that
    /// tall" rather than replacing one with the other.
    fn layout_request(&self) -> Option<LayoutRequest> {
        let mut req = self.request.unwrap_or_else(LayoutRequest::new);
        req = req.stack(self.axis, self.gap);
        if let Some(a) = self.main_align {
            req = req.main_align(a);
        }
        if let Some(a) = self.cross_align {
            req = req.cross_align(a);
        }
        Some(req)
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// Scroll
// ---------------------------------------------------------------------------

/// A viewport that scrolls its content vertically or horizontally.
///
/// ## What it does, and what it deliberately does not
///
/// The wheel scrolls, the content is clipped to the viewport, and the offset is
/// clamped to the content. That is the whole contract. There is no scrollbar:
/// drawing one means deciding its thickness, its track's insets, whether it
/// fades, and whether it takes pointer input — four decisions that belong to a
/// design system, and a `Scroll` in the primitive layer has none. A kit that
/// wants a bar composes one over this.
///
/// There is no keyboard scrolling either, for a different reason: which element
/// a PageDown belongs to is a focus question — the focused thing inside the
/// scroll area, the scroll area itself, or the nearest scrollable ancestor —
/// and there is no answer to that without a focus policy. The wheel has no such
/// ambiguity: the event is routed by position, so the container under the
/// pointer is the one that scrolls.
///
/// ## Why the offset lives here and not in a signal
///
/// A signal would be a second source of truth. The host already stores the
/// applied offset on the node and applies it during layout, so a signal would
/// have to be kept in step with the host by every writer, and the two would
/// disagree the first time a container was rebuilt. A plain field cannot drift:
/// there is exactly one copy, and it is the one the host was last told.
///
/// ## The content size is one frame old
///
/// Clamping needs the content's extent, and only the host knows it. It is
/// measured during `update` — which runs before layout, so it is the *previous*
/// frame's geometry, cached here on the way past. That is the latest figure that
/// exists at the moment a wheel event arrives, and a frame of lag on a size that
/// has not been re-measured is invisible: nothing about the content has changed,
/// so the clamp it would compute is the clamp it did compute.
///
/// A widget that resized its content in the same frame it was scrolled gets the
/// new clamp on the next wheel turn, which is a frame nobody can perceive
/// because the wheel has stopped by then.
#[derive(Debug, Clone)]
pub struct Scroll {
    /// The content being scrolled. Usually one, but a scroll area is a
    /// container, and a container that cannot hold anything is not one — the
    /// content is what gives it a size to clamp against, and a `Scroll` with no
    /// children has none.
    children: Vec<Child>,
    /// Which axes the wheel may scroll. A horizontal-only strip must not
    /// scroll vertically when the wheel rolls over it, and a vertical list must
    /// not drift sideways on a trackpad's horizontal swipe.
    axes: ScrollAxes,
    /// Where the viewport is, shared rather than a field.
    ///
    /// A composite widget is rebuilt whenever its parent re-declares
    /// `children()`, and a rebuild constructs a fresh value — so a plain field
    /// would silently reset to the top every time an unrelated part of the tree
    /// changed. The host keeps the *node* across rebuilds, keyed by
    /// [`ChildKey`], but not the widget value that describes it.
    ///
    /// This is the same shape [`Accordion`] uses for its open set and the same
    /// reason: the state has to outlive the value that reads it. The host's copy
    /// of the applied offset stays the authority — this is where the widget
    /// remembers what it asked for.
    offset: Arc<Mutex<ScrollOffset>>,
    /// The content extent measured on the last `update`, in the same units as
    /// [`ScrollOffset`]. `None` until the host has measured once.
    content: Option<(f64, f64)>,
}

/// Which axes a [`Scroll`] responds to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollAxes {
    pub horizontal: bool,
    pub vertical: bool,
}

impl Default for ScrollAxes {
    fn default() -> Self {
        Self {
            horizontal: false,
            vertical: true,
        }
    }
}

impl Scroll {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            axes: ScrollAxes::default(),
            offset: Arc::new(Mutex::new(ScrollOffset::ZERO)),
            content: None,
        }
    }

    /// The content this viewport scrolls.
    pub fn child(mut self, child: Child) -> Self {
        self.children.push(child);
        self
    }

    /// Scrolls on both axes. A trackpad is two-dimensional and a pane whose
    /// content is both wider and taller than its viewport needs both.
    pub fn both_axes() -> Self {
        Self {
            axes: ScrollAxes {
                horizontal: true,
                vertical: true,
            },
            ..Self::new()
        }
    }

    /// Scrolls sideways only.
    pub fn horizontal() -> Self {
        Self {
            axes: ScrollAxes {
                horizontal: true,
                vertical: false,
            },
            ..Self::new()
        }
    }

    /// The axes this container responds to.
    pub fn axes(mut self, axes: ScrollAxes) -> Self {
        self.axes = axes;
        self
    }

    /// Where the viewport currently is.
    pub fn offset(&self) -> ScrollOffset {
        *self.offset.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The content extent this container last saw, in pixels.
    ///
    /// `None` until the host has measured once, which is also what makes a
    /// fresh container refuse to move: with nothing to clamp against, every
    /// offset would be past the end.
    pub fn content_extent(&self) -> Option<(f64, f64)> {
        self.content
    }

    /// Records the measured content extent.
    ///
    /// Called by the host through [`UpdateContext::content_size`] on every
    /// update, and public because a caller that lays a container out by hand
    /// needs the same door: the wheel cannot clamp against an extent nobody
    /// told it.
    pub fn with_content_extent(mut self, extent: Size) -> Self {
        self.content = Some((extent.width as f64, extent.height as f64));
        self
    }

    /// Moves the viewport without a wheel, clamped the same way.
    ///
    /// Returns whether the offset actually moved, which is `false` at either end
    /// — the same signal [`Widget::event`] gives back when it declines to
    /// consume a wheel, so a caller can pass an unconsumed gesture on.
    pub fn scroll_to(&mut self, offset: ScrollOffset, viewport: (f64, f64)) -> bool {
        let next = ::floom_core::clamp_offset(
            offset,
            (
                if self.axes.horizontal { viewport.0 } else { f64::INFINITY },
                if self.axes.vertical { viewport.1 } else { f64::INFINITY },
            ),
            (
                if self.axes.horizontal {
                    self.content.map(|c| c.0).unwrap_or(0.0)
                } else {
                    f64::INFINITY
                },
                if self.axes.vertical {
                    self.content.map(|c| c.1).unwrap_or(0.0)
                } else {
                    f64::INFINITY
                },
            ),
        );
        let mut current = self.offset.lock().unwrap_or_else(|e| e.into_inner());
        let moved = next != *current;
        *current = next;
        moved
    }

    /// The axes this container responds to.
    pub fn scroll_axes(&self) -> ScrollAxes {
        self.axes
    }

    /// Whether the content currently extends past the viewport on either axis.
    pub fn is_scrollable(&self) -> bool {
        self.content.is_some_and(|(w, h)| (w > 0.0 && h > 0.0) && (w != h))
    }

    /// The largest offset the content allows, given a viewport of
    /// `viewport`. `None` before the host has measured the content.
    pub fn max_offset(&self, viewport: (f64, f64)) -> Option<ScrollOffset> {
        let (w, h) = self.content?;
        Some(::floom_core::max_offset(
            (
                if self.axes.horizontal { viewport.0 } else { f64::INFINITY },
                if self.axes.vertical { viewport.1 } else { f64::INFINITY },
            ),
            (
                if self.axes.horizontal { w } else { f64::INFINITY },
                if self.axes.vertical { h } else { f64::INFINITY },
            ),
        ))
    }

    /// Where a wheel delta of `delta` would put the viewport, given a viewport
    /// of `viewport`.
    ///
    /// An axis this container does not scroll is passed as `INFINITY`, so
    /// `max_offset` sees no overflow on it and the clamp leaves the offset
    /// alone. That is why the flags go into the arithmetic rather than into a
    /// branch afterwards: one clamp, with the disabled axis made harmless.
    ///
    /// Pure: it does not move the viewport. Clamping is arithmetic on two
    /// numbers, so it is computed here rather than by calling [`Self::scroll_to`]
    /// on a copy — a copy shares the offset cell, and "previewing" a scroll by
    /// performing it is the kind of thing that shows up as a viewport that jumps
    /// when nothing scrolled it.
    pub fn offset_after_wheel(&self, delta: (f64, f64), viewport: (f64, f64)) -> ScrollOffset {
        let here = self.offset();
        let limit = self.max_offset(viewport).unwrap_or(ScrollOffset::ZERO);
        floom_core::clamp_offset(
            ScrollOffset::new(here.x + delta.0, here.y + delta.1),
            (
                if self.axes.horizontal { viewport.0 } else { f64::INFINITY },
                if self.axes.vertical { viewport.1 } else { f64::INFINITY },
            ),
            (
                // Unmeasured content counts as *zero-sized*, not as infinitely
                // large. `INFINITY` here would leave the clamp with nothing to
                // bite on and let a fresh container scroll forever — the one
                // state in which there is provably nothing to scroll to. An
                // axis this container does not scroll is still `INFINITY`, which
                // is what makes the clamp leave it alone.
                if self.axes.horizontal {
                    self.content.map(|c| c.0).unwrap_or(0.0)
                } else {
                    f64::INFINITY
                },
                if self.axes.vertical {
                    self.content.map(|c| c.1).unwrap_or(0.0)
                } else {
                    f64::INFINITY
                },
            ),
        )
    }
}

impl Default for Scroll {
    fn default() -> Self {
        Self::new()
    }
}

impl WidgetAny for Scroll {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        Some(floom_core::SemanticsHint {
            role: floom_core::SemanticsRole::Scrollable,
            ..floom_core::SemanticsHint::default()
        })
    }
}

impl Widget for Scroll {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    /// The content, declared like any other composite's children.
    ///
    /// The host lays it out in this container's own coordinates and applies the
    /// scroll offset afterwards, so the content's size is whatever it turns out
    /// to be rather than whatever this container's box implies.
    fn children(&self) -> Vec<Child> {
        self.children.clone()
    }

    /// Cache the content extent on the way past.
    ///
    /// The host measures this node because `needs_content_size` says so, and
    /// only widgets that opted in are measured — a subtree walk per node would
    /// put an allocation-free but O(subtree) cost on every frame of every tree.
    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.content = cx
            .content_size(id)
            .map(|s| (s.width as f64, s.height as f64));
    }

    /// Clipped to its own box.
    ///
    /// Without this the content paints outside the viewport and a scroll
    /// container stops looking like a container: its rows draw over whatever is
    /// beside it, which is the one thing a scroll area is supposed to prevent.
    fn clip(&self) -> Option<floom_types::ClipShape> {
        Some(floom_types::ClipShape::Box)
    }

    fn needs_content_size(&self) -> bool {
        true
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        let Event::Scroll(delta) = cx.event() else {
            return false;
        };
        // The viewport is this element's own box, which the host has already
        // resolved. An element with no box has no viewport to scroll.
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return false;
        }
        // Read here rather than from the extent cached in `update`: a wheel can
        // arrive before this node has ever been updated, and a container that
        // clamped against a stale figure would stop at the wrong end. The host
        // measured it on the way in.
        let Some(content) = cx.content_size(id) else {
            // Nothing measured, so nothing to scroll. Passing the wheel on beats
            // refusing it — a page behind this container may well scroll.
            return false;
        };
        self.content = Some((content.width as f64, content.height as f64));

        let viewport = (bounds.width(), bounds.height());
        let next = self.offset_after_wheel((delta.delta_x, delta.delta_y), viewport);
        if next == self.offset() {
            // Already at an end. Swallowing the wheel anyway would stop a nested
            // scroller inside this one from ever being reached, so an exhausted
            // scroll passes the wheel down rather than eating it.
            return false;
        }
        self.scroll_to(next, viewport);
        cx.request_scroll(next);
        true
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        // Announced as scrollable even when there is nothing to scroll: assistive
        // technology is told what the widget *is*, and a container that becomes
        // scrollable later should not have to re-announce itself. The hint
        // itself comes from `WidgetAny`, which is where a mount point reads it.
        crate::primitives::emit_semantics(
            SemanticsHint {
                role: SemanticsRole::Scrollable,
                ..SemanticsHint::default()
            },
            cx,
        );
    }
}

// ---------------------------------------------------------------------------
// Conditional
// ---------------------------------------------------------------------------

/// Shows its single child when `when` is true, otherwise renders nothing.
///
/// Toggling `when` mounts/unmounts the subtree without touching its siblings.
/// With [`Self::enter`] / [`Self::exit`] set, the subtree fades (or grows, or
/// slides) in and out instead of snapping: an exiting subtree is kept mounted
/// until its exit animation has finished, so the departure is visible rather
/// than an instant disappearance. A `Conditional` with no transitions behaves
/// exactly as before — a plain mount/unmount.
#[derive(Clone)]
pub struct Conditional {
    when: bool,
    last_when: bool,
    child: Option<Child>,
    /// The transition played as the child appears. `None` snaps it in.
    enter: Option<PresenceTransition>,
    /// The transition played as the child leaves. `None` snaps it out (the
    /// child unmounts on the next frame).
    exit: Option<PresenceTransition>,
    /// What the child is doing right now.
    phase: PresencePhase,
    /// Bumped every time a transition is (re-)armed, so the host restarts it
    /// even when the same `from`/`to`/`duration` is declared again — a child
    /// that re-appears must replay its enter, not resume a finished one.
    rev: u64,
    /// Set by the host when a declared transition finishes; the next `update`
    /// folds its end into the tree (unmount a finished exit, rest a finished
    /// enter).
    finished: bool,
    /// A shared flag to follow instead of a fixed `when`.
    ///
    /// A kit whose visibility is driven from outside -- a button that opens a
    /// panel -- cannot hold an `RwSignal` in the tree, because a slot closure
    /// has to be `Send + Sync` and an `RwSignal` is `Rc`-based. The flag is the
    /// same `Arc<Mutex<bool>>` the text input's buffer uses, and it is read in
    /// `update`, so the tree follows whoever writes it.
    flag: Option<crate::text_input::SharedFlag>,
}

/// Where a presence child is in its life cycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PresencePhase {
    /// Not mounted.
    Hidden,
    /// An enter transition is playing (or snapping straight to rest).
    Entering,
    /// At rest, fully shown.
    Shown,
    /// An exit transition is playing while the child stays mounted.
    Exiting,
}

impl std::fmt::Debug for Conditional {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Conditional")
            .field("when", &self.when)
            .field("phase", &self.phase)
            .finish()
    }
}

impl Conditional {
    pub fn new(when: bool) -> Self {
        // Start "changed" so the first frame always reconciles, and seed the
        // phase from `when` so a child that starts present fades in rather than
        // appearing pre-faded.
        let phase = if when {
            PresencePhase::Entering
        } else {
            PresencePhase::Hidden
        };
        Self {
            when,
            last_when: !when,
            child: None,
            enter: None,
            exit: None,
            phase,
            rev: 0,
            finished: false,
            flag: None,
        }
    }

    /// Follows a shared flag rather than a fixed boolean.
    ///
    /// The flag is read in `update`, so whoever writes it -- a button, a route,
    /// a test -- drives the child in and out. The flag's value at the moment
    /// this is built seeds the first frame, so a conditional that starts open
    /// is open immediately rather than a frame later.
    pub fn when_flag(mut self, flag: crate::text_input::SharedFlag) -> Self {
        self.when = crate::text_input::read_flag(&flag);
        self.last_when = !self.when;
        self.flag = Some(flag);
        self
    }

    /// Sets the (single) child shown while `when` is true.
    pub fn show(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    /// Sets the (single) child with an explicit key and layout.
    pub fn show_with(
        mut self,
        key: impl Into<ChildKey>,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.child = Some(Child::new(key, layout, build));
        self
    }

    /// The transition played as the child appears.
    pub fn enter(mut self, transition: PresenceTransition) -> Self {
        self.enter = Some(transition);
        self
    }

    /// The transition played as the child appears, if there is one.
    pub fn enter_transition(&self) -> Option<PresenceTransition> {
        self.enter
    }

    /// The transition played as the child leaves (and before it unmounts).
    pub fn exit(mut self, transition: PresenceTransition) -> Self {
        self.exit = Some(transition);
        self
    }

    /// The transition played as the child leaves, if there is one.
    pub fn exit_transition(&self) -> Option<PresenceTransition> {
        self.exit
    }

    /// Convenience for symmetric enter/exit transitions.
    pub fn presence(mut self, transition: PresenceTransition) -> Self {
        self.enter = Some(transition);
        self.exit = Some(transition);
        self
    }

    /// Updates whether the child is shown; the next frame reconciles.
    pub fn set_when(&mut self, when: bool) {
        self.when = when;
    }

    /// Adopts `when` as the state this widget *starts* from, without treating it
    /// as a change.
    ///
    /// The distinction is load-bearing. [`Self::new`] deliberately starts "not
    /// yet seen" so the first frame reconciles — which means a
    /// `Conditional::new(false)` reads its own starting state as a *departure*
    /// on the first `update`, and would play the exit transition for a child
    /// that was never there. A parent that already knows the answer (a
    /// disclosure panel seeded from its shared open flag) says so here instead.
    /// Sets the flag from outside the widget.
    ///
    /// Public because a composite above this crate drives the presence of a leaf
    /// it owns -- the accordion panel, the dialog body -- and has to be able to say
    /// "show this" without wrapping the leaf in another widget first.
    pub fn sync_when(&mut self, when: bool) {
        self.when = when;
        self.last_when = when;
        self.phase = if when {
            PresencePhase::Shown
        } else {
            PresencePhase::Hidden
        };
        self.finished = false;
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Widget for Conditional {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // The host finished a transition last frame: fold its end into the
        // tree now, in update, where marking dirty actually takes effect.
        if self.finished {
            self.finished = false;
            match self.phase {
                PresencePhase::Entering => {
                    // The child is already present and at rest; only a repaint
                    // is owed, and cancelling the declaration returns it to its
                    // un-composited (full) appearance.
                    self.phase = PresencePhase::Shown;
                    cx.mark_dirty(id);
                }
                PresencePhase::Exiting => {
                    // The departure has played; the subtree can really unmount.
                    self.phase = PresencePhase::Hidden;
                    cx.mark_children_dirty(id);
                    cx.mark_dirty(id);
                }
                _ => {}
            }
            return;
        }

        // A conditional that follows a flag reads it here, so the tree follows
        // whoever writes it.
        if let Some(flag) = self.flag.clone() {
            self.when = crate::text_input::read_flag(&flag);
        }

        if self.when != self.last_when {
            self.last_when = self.when;
            // Whether or not a transition plays, the child's mount state
            // changes, so the subtree must reconcile.
            cx.mark_children_dirty(id);
            cx.mark_dirty(id);
            if self.when {
                // Appearing: snap to rest when there is no enter transition,
                // otherwise arm one (its restart key bumps so a re-appearance
                // replays it).
                match self.enter {
                    Some(transition) if !transition.is_instant() => {
                        self.phase = PresencePhase::Entering;
                        self.rev = self.rev.wrapping_add(1);
                    }
                    _ => self.phase = PresencePhase::Shown,
                }
            } else {
                match self.exit {
                    Some(transition) if !transition.is_instant() => {
                        self.phase = PresencePhase::Exiting;
                        self.rev = self.rev.wrapping_add(1);
                    }
                    _ => self.phase = PresencePhase::Hidden,
                }
            }
        }
    }

    fn children(&self) -> Vec<Child> {
        // The child is mounted while present *and* while exiting: an exit
        // transition can only play while the subtree is still on screen. Hidden
        // is the only phase with no child.
        match &self.child {
            Some(child) if self.phase != PresencePhase::Hidden => vec![child.clone()],
            _ => Vec::new(),
        }
    }

    fn animation(&self) -> Option<floom_core::AnimationSpec> {
        let spec = match self.phase {
            PresencePhase::Entering => self.enter?.spec(true),
            PresencePhase::Exiting => self.exit?.spec(false),
            _ => return None,
        };
        Some(spec.with_key(self.rev))
    }

    fn animation_finished(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {
        // A presence transition ended. Acting here would unmount a subtree from
        // inside the host's tick, so we only flag and let `update` fold it in.
        self.finished = true;
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// PresenceLeaf
// ---------------------------------------------------------------------------

/// A child whose `when` is driven by a shared flag rather than internal state,
/// so a parent (a [`Crossfade`]) can flip it without holding the child's node.
///
/// Everything else delegates to an inner [`Conditional`] — this is purely the
/// "read `when` from a signal" seam that lets a composite own the choreography
/// while the child owns the presence lifecycle it already implements.
#[derive(Clone)]
pub(crate) struct PresenceLeaf {
    inner: Conditional,
    when_signal: Arc<AtomicBool>,
}

impl PresenceLeaf {
    /// A presence leaf publishes whatever its inner [`Conditional`] does.
    pub fn semantics_hint(&self) -> SemanticsHint {
        self.inner.semantics_hint()
    }
}

impl Widget for PresenceLeaf {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let want = self.when_signal.load(Ordering::Relaxed);
        self.inner.set_when(want);
        self.inner.update(cx, id);
    }

    fn children(&self) -> Vec<Child> {
        self.inner.children()
    }

    fn animation(&self) -> Option<floom_core::AnimationSpec> {
        self.inner.animation()
    }

    fn animation_finished(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.inner.animation_finished(cx, id);
    }

    fn paint(&self, cx: &mut dyn PaintContext, id: ElementId) {
        self.inner.paint(cx, id);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, id: ElementId) {
        self.inner.semantics(cx, id);
    }
}

// ---------------------------------------------------------------------------
// Crossfade
// ---------------------------------------------------------------------------

/// Swaps between two contents by crossfading them, instead of cutting.
///
/// Both contents stay mounted — following Flutter's `AnimatedCrossFade`, which
/// keeps both children live — and are stacked in the same box, so the one
/// leaving plays its exit transition at the same moment the one arriving plays
/// its enter. Each side is a presence child in its own right, so a departure is
/// visible: the outgoing content stays on screen until its motion ends, it does
/// not vanish the instant the swap begins.
///
/// [`Crossfade::presence`] chooses the transition (a fade by default); a
/// non-positive duration collapses to an instant swap, the same as a
/// `Conditional` with no transition.
#[derive(Clone)]
pub struct Crossfade {
    show_a: bool,
    last_show_a: bool,
    a: Option<Child>,
    b: Option<Child>,
    transition: PresenceTransition,
    signal_a: Arc<AtomicBool>,
    signal_b: Arc<AtomicBool>,
}

impl Crossfade {
    /// Starts showing `a` (`show_a = true`) or `b` (`show_a = false`).
    pub fn new(show_a: bool) -> Self {
        Self {
            show_a,
            last_show_a: show_a,
            a: None,
            b: None,
            // Instant until a transition is supplied: an unconfigured crossfade
            // is just a swap, as a `Conditional` with no transition is.
            transition: PresenceTransition::fade(0.0),
            signal_a: Arc::new(AtomicBool::new(show_a)),
            signal_b: Arc::new(AtomicBool::new(!show_a)),
        }
    }

    /// The first content, shown while `show_a` is true.
    pub fn a(mut self, child: Child) -> Self {
        self.a = Some(child);
        self
    }

    /// The second content, shown while `show_a` is false.
    pub fn b(mut self, child: Child) -> Self {
        self.b = Some(child);
        self
    }

    /// The transition each side plays as it arrives or leaves. A fade by
    /// default; a non-positive duration is treated as an instant swap.
    pub fn presence(mut self, transition: PresenceTransition) -> Self {
        self.transition = transition;
        self
    }

    /// Swap which content is shown; the next frame crossfades.
    pub fn set_show_a(&mut self, show_a: bool) {
        self.show_a = show_a;
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Widget for Crossfade {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {
        if self.show_a != self.last_show_a {
            self.last_show_a = self.show_a;
            self.signal_a.store(self.show_a, Ordering::Relaxed);
            self.signal_b.store(!self.show_a, Ordering::Relaxed);
            // Deliberately no `mark_dirty` here: nothing about *this* widget
            // changed — it still owns the same two slots. What changed is each
            // leaf's `when`, and every leaf reads its own signal and reports
            // that change itself (that is `Conditional::update`, which both
            // marks the node dirty and re-syncs the declared transition, since
            // the host only re-syncs declarations for dirty nodes). Marking
            // dirty here too would be a second report of a change already
            // reported, and it is not observable — the S5 mutation campaign
            // removes it and nothing moves.
        }
    }

    fn children(&self) -> Vec<Child> {
        let mut out = Vec::new();
        if let Some(a) = &self.a {
            let leaf = PresenceLeaf {
                inner: Conditional::new(self.signal_a.load(Ordering::Relaxed))
                    .show(a.clone())
                    .presence(self.transition),
                when_signal: self.signal_a.clone(),
            };
            out.push(Child::new(
                ChildKey::index(0),
                LayoutRequest::new().absolute_fill(),
                move || Box::new(leaf.clone()),
            ));
        }
        if let Some(b) = &self.b {
            let leaf = PresenceLeaf {
                inner: Conditional::new(self.signal_b.load(Ordering::Relaxed))
                    .show(b.clone())
                    .presence(self.transition),
                when_signal: self.signal_b.clone(),
            };
            out.push(Child::new(
                ChildKey::index(1),
                LayoutRequest::new().absolute_fill(),
                move || Box::new(leaf.clone()),
            ));
        }
        out
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// SharedTransitionLayout
// ---------------------------------------------------------------------------

/// What the shared element is doing right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SharedPhase {
    /// Showing the source, at rest.
    Source,
    /// The source's ink is flying toward the destination's box.
    Flying,
    /// Showing the destination, at rest (the flight landed).
    Destination,
}

/// Flies one of two keyed subtrees ("shared elements") into the other's box,
/// the shared-element transition Compose calls `SharedTransitionLayout` and
/// Flutter calls `Hero`.
///
/// Both subtrees stay mounted (each as a presence child in its own leaf), so
/// the swap is a *flight* rather than a cut: while `fly` is armed, the source
/// keeps painting and the whole layout translates it from its box toward the
/// destination's — same coordinates the destination would claim — and only
/// when the flight lands is the destination's own content revealed. The
/// destination is laid out `delta` pixels below the source, so the translate
/// (a single `TranslateY` channel, B6) lands exactly on it.
///
/// Interrupting the flight — pointing the translate somewhere else mid-run —
/// goes through the host's retarget door, so it keeps the speed it had (the
/// scheduler turns the tween into a momentum-carrying spring) instead of
/// snapping to the new target.
#[derive(Clone)]
pub struct SharedTransitionLayout {
    /// The shared-element identity; exposed so a host can name the pair.
    key: u64,
    fly: bool,
    last_fly: bool,
    phase: SharedPhase,
    /// Whether the current flight travels source→destination (or back).
    flying_forward: bool,
    /// Destination's offset from the source, in logical pixels.
    delta: f32,
    duration: f64,
    easing: Easing,
    /// Bumped every time a flight is (re-)armed so the host restarts the
    /// declaration even when the same spec comes back.
    rev: u64,
    /// Set by the host when a flight ends; the next `update` folds it in.
    finished: bool,
    source: Option<Child>,
    dest: Option<Child>,
    signal_source: Arc<AtomicBool>,
    signal_dest: Arc<AtomicBool>,
    /// Optional external gate that overrides [`Self::set_fly`], so a caller
    /// (a test, an app) can arm a flight on an already-mounted layout by
    /// storing into a handle it kept — the same seam `PresenceLeaf` gives a
    /// parent over its children.
    fly_signal: Option<Arc<AtomicBool>>,
}

impl SharedTransitionLayout {
    /// A shared layout with `key`, whose destination sits `delta` (logical
    /// px) below its source.
    pub fn new(key: u64, delta: f32) -> Self {
        Self {
            key,
            fly: false,
            last_fly: false,
            phase: SharedPhase::Source,
            flying_forward: true,
            delta,
            duration: 0.3,
            easing: Easing::Linear,
            rev: 0,
            finished: false,
            source: None,
            dest: None,
            signal_source: Arc::new(AtomicBool::new(true)),
            signal_dest: Arc::new(AtomicBool::new(false)),
            fly_signal: None,
        }
    }

    /// The source subtree, shown at rest and flown while flying.
    pub fn source(mut self, child: Child) -> Self {
        self.source = Some(child);
        self
    }

    /// The destination subtree, revealed once the flight lands.
    pub fn dest(mut self, child: Child) -> Self {
        self.dest = Some(child);
        self
    }

    /// The duration of the flight.
    pub fn duration(mut self, duration: f64) -> Self {
        self.duration = duration;
        self
    }

    /// Arms (`true`) the flight to the destination, or back (`false`).
    pub fn set_fly(&mut self, fly: bool) {
        self.fly = fly;
    }

    /// Routes fly decisions through `signal`: the layout reads it every update
    /// instead of the value [`Self::set_fly`] last wrote, so a caller that kept
    /// the handle can arm a flight on an already-mounted layout.
    pub fn fly_signal(mut self, signal: Arc<AtomicBool>) -> Self {
        self.fly_signal = Some(signal);
        self
    }

    /// The shared-element identity.
    pub fn shared_key(&self) -> u64 {
        self.key
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Widget for SharedTransitionLayout {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // A flight ended last frame: fold its landing in now, where marking
        // children dirty actually takes effect.
        if self.finished {
            self.finished = false;
            self.phase = if self.flying_forward {
                SharedPhase::Destination
            } else {
                SharedPhase::Source
            };
            self.sync_signals();
            cx.mark_children_dirty(id);
            cx.mark_dirty(id);
            return;
        }

        // Only declarations are reconciled here, but the armed flight is read
        // fresh each update so an external signal can flip it on a mounted
        // layout.
        if let Some(signal) = &self.fly_signal {
            self.fly = signal.load(Ordering::Relaxed);
        }
        if self.fly != self.last_fly {
            self.last_fly = self.fly;
            let forward = self.fly && self.phase == SharedPhase::Source
                || !self.fly && self.phase == SharedPhase::Destination;
            if forward {
                self.phase = SharedPhase::Flying;
                self.flying_forward = self.fly;
                self.rev = self.rev.wrapping_add(1);
                self.sync_signals();
                // Only this widget changed (the leaves stay mounted), so a
                // repaint — which re-syncs the declaration — is all that's owed.
                cx.mark_dirty(id);
            }
        }
    }

    fn children(&self) -> Vec<Child> {
        let mut out = Vec::new();
        if let Some(source) = &self.source {
            let leaf = PresenceLeaf {
                inner: Conditional::new(self.signal_source.load(Ordering::Relaxed))
                    .show(source.clone()),
                when_signal: self.signal_source.clone(),
            };
            // Both leaves are anchored absolutely to the layout's box origin so
            // their global positions are fixed: source at the top-left, the
            // destination `delta` below it. The flight translate then moves the
            // source's ink from the one box onto the other.
            out.push(Child::new(
                ChildKey::index(0),
                absolute_layout(0.0),
                move || Box::new(leaf.clone()),
            ));
        }
        if let Some(dest) = &self.dest {
            let leaf = PresenceLeaf {
                inner: Conditional::new(self.signal_dest.load(Ordering::Relaxed))
                    .show(dest.clone()),
                when_signal: self.signal_dest.clone(),
            };
            out.push(Child::new(
                ChildKey::index(1),
                absolute_layout(self.delta),
                move || Box::new(leaf.clone()),
            ));
        }
        out
    }

    fn animation(&self) -> Option<floom_core::AnimationSpec> {
        if self.phase != SharedPhase::Flying {
            return None;
        }
        let delta = self.delta as f64;
        let (from, to) = if self.flying_forward {
            (0.0, delta)
        } else {
            (0.0, -delta)
        };
        Some(
            floom_core::AnimationSpec::new(from, to, self.duration)
                .with_easing(self.easing)
                .with_key(self.rev)
                .with_prop(CompositorProp::TranslateY(0.0)),
        )
    }

    fn animation_finished(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {
        self.finished = true;
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

impl WidgetAny for SharedTransitionLayout {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        Some(<Self>::semantics_hint(self).into())
    }
}

impl SharedTransitionLayout {
    fn sync_signals(&mut self) {
        let (source, dest) = match self.phase {
            SharedPhase::Source => (true, false),
            SharedPhase::Destination => (false, true),
            SharedPhase::Flying => {
                if self.flying_forward {
                    (true, false)
                } else {
                    (false, true)
                }
            }
        };
        self.signal_source.store(source, Ordering::Relaxed);
        self.signal_dest.store(dest, Ordering::Relaxed);
    }
}

/// A child pinned to the layout's top-left corner (`top` px down the layout
/// box, so `top = 0` is the source box and `top = delta` the destination's).
fn absolute_layout(top: f32) -> LayoutRequest {
    LayoutRequest {
        position: Some(Position::Absolute),
        inset: Some(Inset {
            left: Some(0.0),
            top: Some(top),
            right: None,
            bottom: None,
        }),
        ..LayoutRequest::new()
    }
}

// ---------------------------------------------------------------------------
// AnimatedContent
// ---------------------------------------------------------------------------

/// A content-keyed presence transition: whenever the tracked `value` changes,
/// the outgoing content stays mounted and plays its exit while the incoming
/// content plays its enter. [`Crossfade`] is the two-value special case; this
/// is the N-ary, key-driven generalisation Flutter calls `AnimatedSwitcher`.
///
/// The transition is supplied once with [`AnimatedContent::presence`]; every
/// side plays it. A non-positive duration collapses to an instant swap. The
/// defining property is the one [`Crossfade`] has to keep too: the leaving
/// content must remain on screen until its exit animation is over, or the
/// departure is invisible.
///
/// `T` is the *content key* — it decides, by `Eq`, when a switch has happened,
/// and the content for each key comes from the closure passed to
/// [`AnimatedContent::content_with`]. Two simultaneously-live keys (the leaving
/// and the arriving) are therefore two distinct subtrees, each animating on its
/// own.
#[derive(Clone)]
pub struct AnimatedContent<T: Clone + Eq + Send + Sync + 'static> {
    value: T,
    last_value: T,
    build: Option<Arc<dyn Fn(&T) -> Child + Send + Sync>>,
    transition: PresenceTransition,
    /// One live subtree per currently-mounted-or-exiting value. Each gets a
    /// unique, ever-increasing id used as its [`ChildKey`], so two different
    /// values never collide and a value that returns later mounts a fresh node
    /// rather than resurrecting a finished one.
    entries: Vec<AnimatedContentSlot>,
    next_id: u64,
    /// The id of the entry currently shown at rest (the one *not* exiting).
    /// `None` until the first content is mounted.
    active_id: Option<u64>,
}

/// A live subtree for one value, plus the bookkeeping that lets the parent
/// prune it once its exit has finished.
#[derive(Clone)]
struct AnimatedContentSlot {
    id: u64,
    leaf: AnimatedContentLeaf,
}

/// A [`PresenceLeaf`] that additionally reports when *its own* exit has
/// completed, so the parent [`AnimatedContent`] can drop the finished subtree
/// instead of leaking a hidden node on every switch.
///
/// Everything but `exited` delegates to the inner [`PresenceLeaf`], which
/// already owns the enter/exit lifecycle; this wrapper only adds the one bit of
/// state the parent needs and the inner leaf cannot see. The content lives in
/// the wrapped [`Conditional`] as a [`Child`], so this wrapper itself carries
/// no `T` — which is why it is not generic.
#[derive(Clone)]
struct AnimatedContentLeaf {
    inner: PresenceLeaf,
    exited: Arc<AtomicBool>,
}

impl AnimatedContentLeaf {
    fn new(child: Child, transition: PresenceTransition, shown: bool) -> Self {
        let signal = Arc::new(AtomicBool::new(shown));
        Self {
            inner: PresenceLeaf {
                inner: Conditional::new(shown).show(child).presence(transition),
                when_signal: signal,
            },
            exited: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Re-points the wrapped transition; used when `presence` is configured
    /// after the initial content was already spawned.
    fn set_transition(&mut self, transition: PresenceTransition) {
        self.inner.inner.enter = Some(transition);
        self.inner.inner.exit = Some(transition);
    }

    fn semantics_hint(&self) -> SemanticsHint {
        self.inner.semantics_hint()
    }
}

impl Widget for AnimatedContentLeaf {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.inner.update(cx, id);
    }

    fn children(&self) -> Vec<Child> {
        self.inner.children()
    }

    fn animation(&self) -> Option<floom_core::AnimationSpec> {
        self.inner.animation()
    }

    fn animation_finished(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // Only an *exiting* leaf may be pruned; the one arriving at rest stays.
        // `when_signal` is false exactly while this leaf is leaving.
        let exiting = !self.inner.when_signal.load(Ordering::Relaxed);
        self.inner.animation_finished(cx, id);
        if exiting {
            self.exited.store(true, Ordering::Relaxed);
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, id: ElementId) {
        self.inner.paint(cx, id);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, id: ElementId) {
        self.inner.semantics(cx, id);
    }
}

impl WidgetAny for AnimatedContentLeaf {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        Some(<Self>::semantics_hint(self).into())
    }
}

impl<T: Clone + Eq + Send + Sync + 'static> AnimatedContent<T> {
    /// Starts tracking `value`; the content for it is supplied by
    /// [`AnimatedContent::content_with`].
    pub fn new(value: T) -> Self {
        Self {
            value: value.clone(),
            last_value: value,
            build: None,
            transition: PresenceTransition::fade(0.0),
            entries: Vec::new(),
            next_id: 0,
            active_id: None,
        }
    }

    /// Produces the child for each value. Called once per switch (and once for
    /// the initial value), so the leaving and arriving subtrees can differ.
    pub fn content_with(mut self, build: impl Fn(&T) -> Child + Send + Sync + 'static) -> Self {
        self.build = Some(Arc::new(build));
        self.spawn_initial();
        self
    }

    /// Convenience for a single static child regardless of value.
    pub fn content(mut self, child: Child) -> Self {
        self.build = Some(Arc::new(move |_| child.clone()));
        self.spawn_initial();
        self
    }

    /// The transition each side plays as it arrives or leaves. A fade by
    /// default; a non-positive duration is treated as an instant swap.
    pub fn presence(mut self, transition: PresenceTransition) -> Self {
        self.transition = transition;
        for slot in &mut self.entries {
            slot.leaf.set_transition(transition);
        }
        self
    }

    /// Switches to `value`; the next frame crossfades if it differs from the
    /// current one.
    pub fn set_value(&mut self, value: T) {
        self.value = value;
    }

    /// Builds and mounts the content for the current `value` as a new, entering
    /// entry. Returns its id, or `None` if no content closure was supplied.
    fn spawn_entry(&mut self) -> Option<u64> {
        let build = self.build.clone()?;
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        let child = build(&self.value);
        let leaf = AnimatedContentLeaf::new(child, self.transition, true);
        self.entries.push(AnimatedContentSlot { id, leaf });
        Some(id)
    }

    /// Mounts the initial entry; called from the content builders so the first
    /// frame already shows content.
    fn spawn_initial(&mut self) {
        if let Some(id) = self.spawn_entry() {
            self.active_id = Some(id);
        }
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl<T: Clone + Eq + Send + Sync + 'static> Widget for AnimatedContent<T> {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // Drop finished exits first, so a long burst of switches cannot leak
        // hidden nodes. The active entry is never pruned.
        if self.entries.len() > 1 {
            let before = self.entries.len();
            self.entries.retain(|slot| {
                slot.id == self.active_id.unwrap_or(u64::MAX)
                    || !slot.leaf.exited.load(Ordering::Relaxed)
            });
            if self.entries.len() != before {
                cx.mark_children_dirty(id);
            }
        }

        // Nothing mounted yet (no content closure, or not yet spawned): remember
        // the value and mount the initial entry if we can.
        if self.entries.is_empty() {
            self.last_value = self.value.clone();
            if let Some(new_id) = self.spawn_entry() {
                self.active_id = Some(new_id);
                cx.mark_children_dirty(id);
            }
            return;
        }

        // A value change: flip the resting entry to exit and spawn the new one.
        if self.value != self.last_value {
            self.last_value = self.value.clone();
            if let Some(active_id) = self.active_id {
                if let Some(slot) = self.entries.iter_mut().find(|s| s.id == active_id) {
                    slot.leaf.inner.when_signal.store(false, Ordering::Relaxed);
                }
            }
            if let Some(new_id) = self.spawn_entry() {
                self.active_id = Some(new_id);
                cx.mark_children_dirty(id);
            }
        }
    }

    fn children(&self) -> Vec<Child> {
        self.entries
            .iter()
            .map(|slot| {
                let leaf = slot.leaf.clone();
                Child::new(
                    ChildKey::keyed(slot.id),
                    LayoutRequest::new().absolute_fill(),
                    move || Box::new(leaf.clone()),
                )
            })
            .collect()
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

impl<T: Clone + Eq + Send + Sync + 'static> WidgetAny for AnimatedContent<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        Some(<Self>::semantics_hint(self).into())
    }
}

// ---------------------------------------------------------------------------
// Opacity — implicit opacity transition (B4 extended from colour to the
// compositor opacity channel)
// ---------------------------------------------------------------------------

/// A single-child wrapper that applies an opacity to its subtree, with an
/// optional implicit transition: change the target and the subtree fades from
/// its old opacity to the new one instead of snapping. This is the B4
/// "change-the-value-and-it-animates" layer extended from colour — which
/// `Button::transition` drives through paint — to the compositor opacity
/// channel: the Flutter `AnimatedOpacity` / Compose `AnimatedVisibility`
/// opacity half.
///
/// When no transition is set, the opacity is *still* applied: a static
/// `Opacity(0.3)` is a real, visible state, not a no-op. The mechanism is the
/// same in both cases — every `Opacity` declares one compositor-driven tween
/// whose endpoints are its old and new value, so the host folds the
/// interpolated alpha onto the whole subtree (and, being a fold-class
/// channel, onto the children too).
#[derive(Clone)]
pub struct Opacity {
    child: Child,
    /// The opacity the subtree is heading toward, `0.0..=1.0`.
    target: f32,
    /// The value the current transition started from — the subtree's previously
    /// rested opacity. Mirrors `Button::transition_from`: a mid-flight reversal
    /// restarts from here rather than from the currently displayed value.
    from: f32,
    /// The target as of the last `update`, compared against `target` to detect
    /// a change worth animating.
    last_target: f32,
    /// The implicit transition shape. `None` means "snap to the new value".
    transition: Option<Transition>,
    /// Bumped every time `target` changes while a transition is armed, so the
    /// host sees a *new* declaration and restarts the tween rather than leaving
    /// the finished one parked.
    rev: u64,
}

impl Opacity {
    /// Wraps `child`, fully opaque by default.
    pub fn new(child: Child) -> Self {
        Self {
            child,
            target: 1.0,
            from: 1.0,
            last_target: 1.0,
            transition: None,
            rev: 0,
        }
    }

    /// Sets the resting opacity. The change is detected on the next `update`;
    /// no animation is armed here.
    pub fn opacity(mut self, value: f32) -> Self {
        let value = value.clamp(0.0, 1.0);
        self.target = value;
        self.from = value;
        self.last_target = value;
        self
    }

    /// Arms the implicit opacity transition: changing the target fades the
    /// subtree from its old opacity to the new one over `transition`.
    pub fn transition(mut self, transition: Transition) -> Self {
        self.transition = Some(transition);
        self
    }

    /// Changes the target opacity at runtime; the next frame animates (or snaps
    /// if no transition is armed).
    pub fn set_opacity(&mut self, value: f32) {
        self.target = value.clamp(0.0, 1.0);
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Widget for Opacity {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        if self.target != self.last_target {
            // A transition starts from the previously rested value. Without a
            // transition the change still needs to reach the compositor, so the
            // key is bumped and the declaration re-synced in every case.
            if self.transition.is_some() {
                self.from = self.last_target;
            }
            self.rev = self.rev.wrapping_add(1);
            self.last_target = self.target;
            // Re-sync the declared animation on this node so the host restarts
            // (or parks the new instant value for) the compositor tween.
            cx.mark_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        vec![self.child.clone()]
    }

    fn animation(&self) -> Option<AnimationSpec> {
        let (from, to, duration) = match self.transition {
            Some(t) => (
                f64::from(self.from),
                f64::from(self.target),
                t.duration_secs,
            ),
            // No transition: an instant tween whose endpoints coincide parks
            // the static value on the compositor.
            None => (f64::from(self.target), f64::from(self.target), 0.0),
        };
        Some(
            AnimationSpec::new(from, to, duration)
                .with_easing(self.transition.map_or(Easing::Linear, |t| t.easing))
                .with_prop(CompositorProp::Opacity(0.0))
                .with_key(self.rev),
        )
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

/// A data-driven list. Mirrors `items` into keyed rows so inserting/removing in
/// the middle keeps each surviving row's widget (scroll position, focus, …)
/// instead of rebuilding the whole list.
///
/// `T` must be `Clone + Send + Sync + 'static` because each row is built from an
/// owned copy of its item inside a `Send + Sync` builder closure.
pub struct List<T> {
    axis: Axis,
    gap: f32,
    items: Vec<T>,
    key_of: Arc<dyn Fn(&T) -> u64 + Send + Sync>,
    build_item: Arc<dyn Fn(T) -> Box<dyn WidgetAny> + Send + Sync>,
    item_layout: LayoutRequest,
    dirty: bool,
}

impl<T: std::fmt::Debug> std::fmt::Debug for List<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("List")
            .field("len", &self.items.len())
            .finish()
    }
}

impl<T: Clone + Send + Sync + 'static> List<T> {
    /// Builds a list from `items`, a stable `key_of` extractor, and an item
    /// builder that turns an owned item into a widget.
    pub fn new(
        axis: Axis,
        items: Vec<T>,
        key_of: impl Fn(&T) -> u64 + Send + Sync + 'static,
        build_item: impl Fn(T) -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        Self {
            axis,
            gap: 0.0,
            items,
            key_of: Arc::new(key_of),
            build_item: Arc::new(build_item),
            item_layout: LayoutRequest::new(),
            dirty: false,
        }
    }

    /// Gap between rows in logical pixels.
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    /// Layout applied to every row (defaults to auto-sized).
    pub fn item_layout(mut self, layout: LayoutRequest) -> Self {
        self.item_layout = layout;
        self
    }

    /// Replaces the backing collection; the next frame reconciles the diff.
    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.dirty = true;
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl<T: Clone + Send + Sync + 'static> Widget for List<T> {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        if self.dirty {
            cx.mark_children_dirty(id);
            self.dirty = false;
        }
    }

    fn children(&self) -> Vec<Child> {
        self.items
            .iter()
            .map(|item| {
                let key = ChildKey::keyed((self.key_of)(item));
                let item = item.clone();
                let build = self.build_item.clone();
                Child::new(key, self.item_layout, move || (build)(item.clone()))
            })
            .collect()
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().stack(self.axis, self.gap))
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

impl<T: Clone + Send + Sync + 'static> WidgetAny for List<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        Some(<Self>::semantics_hint(self).into())
    }
}

// ---------------------------------------------------------------------------
// Grid
// ---------------------------------------------------------------------------

/// A data-driven grid: rows of `columns` keyed cells.
///
/// Where [`List`] mirrors a one-dimensional collection, `Grid` chunks the same
/// collection into fixed-size rows and lays each row out as a [`Stack`] along
/// the horizontal axis. Row keys are a fingerprint of the item keys they hold,
/// so inserting or removing an item remounts only the rows whose membership
/// actually changed — every other row keeps its widgets.
pub struct Grid<T> {
    columns: usize,
    column_gap: f32,
    row_gap: f32,
    items: Vec<T>,
    key_of: Arc<dyn Fn(&T) -> u64 + Send + Sync>,
    build_item: Arc<dyn Fn(T) -> Box<dyn WidgetAny> + Send + Sync>,
    item_layout: LayoutRequest,
    dirty: bool,
}

impl<T: std::fmt::Debug> std::fmt::Debug for Grid<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Grid")
            .field("columns", &self.columns)
            .field("len", &self.items.len())
            .finish()
    }
}

impl<T: Clone + Send + Sync + 'static> Grid<T> {
    /// Builds a grid with `columns` cells per row from `items`.
    pub fn new(
        columns: usize,
        items: Vec<T>,
        key_of: impl Fn(&T) -> u64 + Send + Sync + 'static,
        build_item: impl Fn(T) -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        Self {
            columns: columns.max(1),
            column_gap: 0.0,
            row_gap: 0.0,
            items,
            key_of: Arc::new(key_of),
            build_item: Arc::new(build_item),
            item_layout: LayoutRequest::new(),
            dirty: false,
        }
    }

    /// Sets the horizontal (between columns) and vertical (between rows) gaps.
    pub fn gap(mut self, column_gap: f32, row_gap: f32) -> Self {
        self.column_gap = column_gap;
        self.row_gap = row_gap;
        self
    }

    /// Layout applied to every cell (defaults to auto-sized).
    pub fn item_layout(mut self, layout: LayoutRequest) -> Self {
        self.item_layout = layout;
        self
    }

    /// Replaces the backing collection; the next frame reconciles the diff.
    pub fn set_items(&mut self, items: Vec<T>) {
        self.items = items;
        self.dirty = true;
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl<T: Clone + Send + Sync + 'static> Widget for Grid<T> {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        if self.dirty {
            cx.mark_children_dirty(id);
            self.dirty = false;
        }
    }

    fn children(&self) -> Vec<Child> {
        if self.items.is_empty() {
            return Vec::new();
        }
        self.items
            .chunks(self.columns)
            .enumerate()
            .map(|(row_index, row)| {
                let mut row_stack = Stack::new(Axis::Horizontal).gap(self.column_gap);
                // Fingerprint the row by the keys it holds, so a row whose
                // membership changed remounts while untouched rows survive.
                let mut fingerprint = 0x9E37_79B9_7F4A_7C15u64 ^ row_index as u64;
                for item in row {
                    let key = (self.key_of)(item);
                    fingerprint ^= key;
                    fingerprint = fingerprint.wrapping_mul(0x0000_0100_0000_01b3);
                    let item = item.clone();
                    let build = self.build_item.clone();
                    row_stack = row_stack.child(Child::new(
                        ChildKey::keyed(key),
                        self.item_layout,
                        move || (build)(item.clone()),
                    ));
                }
                Child::new(
                    ChildKey::keyed(fingerprint),
                    LayoutRequest::new().stack(Axis::Horizontal, self.column_gap),
                    move || Box::new(row_stack.clone()),
                )
            })
            .collect()
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().stack(Axis::Vertical, self.row_gap))
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

impl<T: Clone + Send + Sync + 'static> WidgetAny for Grid<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        Some(<Self>::semantics_hint(self).into())
    }
}
