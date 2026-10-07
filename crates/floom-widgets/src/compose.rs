//! Composite (layout + navigation) widgets — Tier-2.
//!
//! These sit above the primitives and describe *structure*: a [`Stack`] arranges
//! children along an axis, [`List`] mirrors a data collection with keyed rows,
//! [`Conditional`] swaps a subtree by a boolean, [`Overlay`]/[`Dialog`] cover
//! their parent with an absolutely positioned layer, and [`Tabs`] shows one of
//! several panels beside a selectable bar.
//!
//! Every composite owns only builders (`Arc<dyn Fn ..>`), never the widgets
//! behind them: [`Widget::children`] hands fresh [`Child`] slots to the host,
//! which diffs them by [`ChildKey`] and reuses mounted widgets across updates.
//! That is what lets a [`List`] keep each row's state when an item is inserted in
//! the middle, and a [`Dialog`] keep focus when it is dismissed and reopened.
//!
//! Composites that change their children at runtime (`List`, `Conditional`,
//! `Overlay`, `Dialog`, `Tabs`) flag the change from [`Widget::update`] by
//! calling [`UpdateContext::mark_children_dirty`]; the host then reconciles just
//! the affected subtree instead of repainting everything.

use std::any::Any;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use floom_accessibility::SemanticsRole;
use floom_core::{
    Align, AnimationSpec, Axis, Child, ChildBuilder, ChildKey, Color, CompositorProp,
    CreateContext, Easing, ElementId, Inset, Insets, LayoutRequest, OverlayHandle, OverlayKind,
    OverlayRequest, PaintContext, Position, PresenceTransition, Rect, SemanticsContext, Transition,
    UpdateContext, Widget, WidgetAny,
};

use crate::controls::{Button, Pressable};
use crate::primitives::{SemanticsHint, Text};

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
        }
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

    /// The transition played as the child leaves (and before it unmounts).
    pub fn exit(mut self, transition: PresenceTransition) -> Self {
        self.exit = Some(transition);
        self
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
    pub(crate) fn sync_when(&mut self, when: bool) {
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
// Overlay-backed surface plumbing
// ---------------------------------------------------------------------------

/// Bookkeeping shared by the composites whose surface belongs *above* the tree.
///
/// These widgets are driven by state — an app flips `present` — so there is no
/// event to react to and the request has to be made from [`Widget::update`].
/// Two outcomes are possible. The host mounts the surface on its overlay stack,
/// which is the normal case and the only one that escapes the parent's clip. Or
/// the host has no overlay support and refuses, in which case the composite
/// falls back to mounting the same surface in-tree — what a headless embedder
/// gets, and what these widgets did before overlays existed.
///
/// ## Why this is public, and why a composite holds it as a field
///
/// It was `pub(crate)` while only [`Popup`] and [`Dialog`] used it, which was
/// correct then: those two *are* the composite, so the bookkeeping lived inside
/// them. It is public now because a composite which is *about* something else —
/// a combobox with a dropdown, a select with a list — has to own the same
/// bookkeeping, and it cannot.
///
/// The reason it cannot is a host fact worth writing down: **unmounting a node
/// does not close the overlays it opened.** `OverlaySlot` is only ever removed by
/// [`Widget::update`]'s `close_overlay` path, so an overlay whose owner is torn
/// down stays on the stack. That rules out the obvious shape — the combobox
/// returns a `Popup` as one of its `children`, and rebuilds it with `present`
/// set — because a child whose presence flag changes has to change key to take
/// effect, and changing key remounts, and remounting orphans the panel.
///
/// So the owner keeps the host as a *field*, flips `present` on it, and forwards
/// [`Widget::overlay_opened`] / [`Widget::overlay_closed`]. Nothing about that is
/// specialised to a combobox, which is why it is a public type rather than code
/// copied into each component.
///
/// The forwarding is the sharp edge, and it has already cost this port once:
/// see `floom_widgets::popup`'s module note on an inherent method shadowing a
/// `Widget` trait method. A composite that defines `overlay_opened` and forgets to
/// forward compiles, runs, opens its panel, and never learns the handle — so it
/// can never close it, and nothing errors.
#[derive(Clone)]
pub struct OverlayHost {
    present: bool,
    last_present: bool,
    /// The ticket the host issued, while the surface sits on its overlay stack.
    handle: Option<OverlayHandle>,
    /// True while the surface is mounted in-tree rather than on the stack.
    in_tree: bool,
}

impl OverlayHost {
    pub fn new(present: bool) -> Self {
        Self {
            present,
            // "Not yet seen": the first `update` always acts on whatever the
            // widget currently is. Starting it at `!present` instead would lose
            // a transition made before the first update — a nested composite is
            // only mounted by the reconciler *after* the update pass that would
            // have run it, so `set_present(true)` in between would be swallowed
            // and the surface would never open.
            last_present: false,
            handle: None,
            in_tree: true,
        }
    }

    /// Whether the surface should be visible at all.
    pub fn is_shown(&self) -> bool {
        self.present
    }

    /// Whether the surface is currently mounted as in-tree children, i.e. the
    /// host refused the overlay (or never had one).
    pub fn takes_children_in_tree(&self) -> bool {
        self.present && self.in_tree
    }

    pub fn set_present(&mut self, present: bool) {
        self.present = present;
    }

    /// Opens or closes this surface's host overlay so it matches `present`.
    ///
    /// `build` runs only while the surface is coming up, and turns the node's
    /// own box plus the host viewport into the request to place — or `None` when
    /// there is nothing to put in an overlay after all (a `Popup` with no
    /// content), in which case nothing is mounted anywhere.
    pub fn reconcile(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        build: impl FnOnce(Rect, Rect) -> Option<OverlayRequest>,
    ) {
        if self.present == self.last_present {
            return;
        }
        let opening = self.present;
        self.last_present = self.present;
        if opening {
            let opened = match build(cx.bounds(), cx.viewport()) {
                Some(request) => cx.open_overlay(request),
                None => false,
            };
            self.in_tree = !opened;
        } else {
            if let Some(handle) = self.handle.take() {
                cx.close_overlay(handle);
            }
            self.in_tree = false;
        }
        // Both outcomes change which subtree exists: taking the host overlay
        // means the in-tree children must go, and a refusal means the in-tree
        // children must come back.
        cx.mark_children_dirty(id);
        cx.mark_dirty(id);
    }

    /// The in-tree layer to mount while there is no host overlay, as a child's
    /// `children()` contribution. Empty in the normal case.
    ///
    /// This is the half of the fallback that used to be written out inside every
    /// composite, and it is here because a composite which is about something else
    /// — a combobox's dropdown — needs the same two lines and would otherwise copy
    /// them: *the fallback covers the control*, because the containing block is the
    /// control's own box.
    ///
    /// That covering is a real limitation and is stated rather than hidden: a
    /// dropdown that should hang **below** its field falls back to covering it, on a
    /// host with no overlay support. The alternative — offsetting within the parent —
    /// is not expressible in [`LayoutRequest`], which offers [`LayoutRequest::
    /// absolute_fill`] and nothing for "below my own bottom edge". A hostless embeder
    /// gets a dropdown on top of its field, which is wrong but visible; the thing it
    /// must not do is show nothing.
    pub fn fallback_children(&self, content: Option<Child>, key: ChildKey) -> Vec<Child> {
        if !self.takes_children_in_tree() {
            return Vec::new();
        }
        let Some(content) = content else {
            return Vec::new();
        };
        let surface = Overlay::new().content_with(key, LayoutRequest::new().fill(), move || {
            content.build()
        });
        vec![Child::new(key, LayoutRequest::new().absolute_fill(), move || {
            Box::new(surface.clone())
        })]
    }

    /// Records the handle the host issued for this surface's overlay.
    pub fn opened(&mut self, handle: OverlayHandle) {
        self.handle = Some(handle);
    }

    /// The overlay carrying this surface came down — because the widget asked it
    /// to, or because the host dismissed it. Reports whether this surface was
    /// the one that closed.
    ///
    /// `present` is cleared as well: a dismissal the widget did not ask for
    /// (clicking outside a popup) has to leave the widget's own state matching
    /// what the user sees, or the next `update` would believe the surface is
    /// still up and never reopen it.
    pub fn closed(&mut self, handle: OverlayHandle) -> bool {
        if self.handle != Some(handle) {
            return false;
        }
        self.handle = None;
        self.present = false;
        self.last_present = false;
        true
    }
}

/// The fallback content for a body that was never supplied.
fn empty_body() -> Child {
    Child::new(ChildKey::index(1), LayoutRequest::new(), || {
        Box::new(Text::new("(empty)"))
    })
}

// ---------------------------------------------------------------------------
// Overlay
// ---------------------------------------------------------------------------

/// An absolutely positioned layer that covers its parent's whole box.
///
/// Use it for modals, drawers and toasts: the child escapes the parent's flow
/// and spans the containing block. Toggling the content mounts/unmounts just
/// the layer, leaving the rest of the tree untouched.
///
/// This is a *layout* primitive, not a host overlay: the layer is clipped by
/// whatever contains it. [`Dialog`], [`Popup`] and [`BottomSheet`] are the
/// composites that hand their surface to the host instead (and fall back to
/// building on this layer when the host has no overlays), which is the
/// difference between "inside this box" and "above everything".
#[derive(Clone)]
pub struct Overlay {
    present: bool,
    last_present: bool,
    child: Option<Child>,
}

impl std::fmt::Debug for Overlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Overlay")
            .field("present", &self.present)
            .finish()
    }
}

impl Overlay {
    pub fn new() -> Self {
        Self {
            present: false,
            last_present: false,
            child: None,
        }
    }

    /// Puts `child` on the layer and marks it present.
    pub fn content(mut self, child: Child) -> Self {
        self.child = Some(child);
        self.present = true;
        self
    }

    /// Puts `build` on the layer with an explicit key and layout.
    pub fn content_with(
        mut self,
        key: impl Into<ChildKey>,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.child = Some(Child::new(key, layout, build));
        self.present = true;
        self
    }

    /// Replaces (or clears) the layer content; `None` dismisses it.
    pub fn set_content(&mut self, child: Option<Child>) {
        self.present = child.is_some();
        self.child = child;
    }

    /// Dismisses the layer.
    pub fn dismiss(&mut self) {
        self.child = None;
        self.present = false;
    }

    /// Whether the layer is up.
    pub fn is_present(&self) -> bool {
        self.present
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Default for Overlay {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Overlay {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        if self.present != self.last_present {
            cx.mark_children_dirty(id);
            self.last_present = self.present;
        }
    }

    fn children(&self) -> Vec<Child> {
        match &self.child {
            Some(child) if self.present => vec![child.clone()],
            _ => Vec::new(),
        }
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().absolute_fill())
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// Dialog
// ---------------------------------------------------------------------------

/// A modal dialog: a centred card rendered above the tree.
///
/// `present` toggles it; while hidden it contributes nothing, so the rest of the
/// tree lays out exactly as if it were not there. Showing it hands the card to
/// the host as a modal overlay, which is what makes it cover the window rather
/// than the box it happens to be declared in — a dialog declared inside a
/// scrolling list is still centred on the screen and still owns the keyboard.
#[derive(Clone)]
pub struct Dialog {
    surface: OverlayHost,
    title: String,
    body: Option<Child>,
    /// The caller's "should this be up" flag, when it has one.
    ///
    /// `None` for a dialog driven purely by [`Self::set_present`], which is what the
    /// engine's own tests and any single-owner caller do. `Some` for a dialog a caller
    /// outside the widget has to drive — which is every ported kit, because v1's
    /// dialogs exposed `open_signal()` "so an external trigger (e.g. a button) can
    /// drive it".
    presence: Option<crate::text_input::SharedFlag>,
}

impl std::fmt::Debug for Dialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dialog")
            .field("present", &self.surface.present)
            .finish()
    }
}

impl Dialog {
    pub fn new() -> Self {
        Self {
            surface: OverlayHost::new(false),
            title: String::new(),
            body: None,
            presence: None,
        }
    }

    /// A dialog whose presence is driven by a shared flag rather than by this widget's
    /// own [`Self::set_present`].
    ///
    /// This is what every ported kit needs, because v1's dialogs handed out their open
    /// signal — `open_signal()`, documented "so an external trigger (e.g. a button) can
    /// drive it" — and an `RwSignal` is `Rc`-based and cannot cross a `Send + Sync` slot
    /// boundary. A shared flag can, and it is the same move as `TextInput`'s buffer and
    /// `TreeView`'s forest.
    ///
    /// The flag is read in `update` and written in `overlay_closed`, so a caller setting
    /// it opens the dialog and the host dismissing the dialog clears it. Both directions
    /// are said out loud, because the alternative — a flag nobody reads back — is a
    /// dialog that comes straight back after the user closed it.
    pub fn with_shared_presence(mut self, presence: crate::text_input::SharedFlag) -> Self {
        self.presence = Some(presence);
        self
    }

    /// The shared presence flag, if this dialog has one.
    pub fn presence(&self) -> Option<crate::text_input::SharedFlag> {
        self.presence.clone()
    }

    /// The text shown at the top of the card.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// The card body (any widget subtree).
    pub fn body(mut self, body: Child) -> Self {
        self.body = Some(body);
        self
    }

    /// The card body built lazily with an explicit key and layout.
    pub fn body_with(
        mut self,
        key: impl Into<ChildKey>,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.body = Some(Child::new(key, layout, build));
        self
    }

    /// Shows or hides the dialog; the next frame reconciles.
    pub fn set_present(&mut self, present: bool) {
        self.surface.set_present(present);
    }

    /// Whether the dialog is up.
    pub fn is_present(&self) -> bool {
        self.surface.is_shown()
    }

    /// The card: a centred, vertically stacked [`Stack`] holding the title and
    /// the body. Built fresh on demand so each mount yields new builders; the
    /// host reuses nodes by key and only mounts/unmounts on `present`.
    fn card(&self) -> Stack {
        let title = self.title.clone();
        let body = self.body.clone().unwrap_or_else(empty_body);
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .align(Align::Center, Align::Center)
            .child(Child::new(
                ChildKey::index(0),
                LayoutRequest::new(),
                move || Box::new(Text::new(title.clone())),
            ))
            .child(body)
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        if self.surface.is_shown() {
            SemanticsHint::labeled(SemanticsRole::Dialog, self.title.clone())
        } else {
            SemanticsHint::decorative()
        }
    }
}

impl Default for Dialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Dialog {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // The caller's flag is the truth about *whether to ask*; `surface.present` is
        // what the bookkeeping acts on. Nothing wires them implicitly — and this exact
        // missing wire cost `Combobox` two bugs in a row.
        if let Some(presence) = self.presence.clone() {
            self.surface
                .set_present(crate::text_input::read_flag(&presence));
        }
        let card = self.card();
        // A dialog covers the window, not its own box, so it is sized from the
        // viewport the host reports rather than from `bounds`.
        self.surface.reconcile(cx, id, move |_bounds, viewport| {
            Some(OverlayRequest::new(
                OverlayKind::Dialog,
                true,
                viewport,
                move |_handle| Box::new(card),
            ))
        });
    }

    fn children(&self) -> Vec<Child> {
        if !self.surface.takes_children_in_tree() {
            return Vec::new();
        }
        // The host refused the overlay (or has none at all), so fall back to the
        // in-tree layer this widget used before it could escape its parent.
        let card = self.card();
        let surface =
            Overlay::new().content_with(ChildKey::index(0), LayoutRequest::new(), move || {
                Box::new(card.clone())
            });
        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new().absolute_fill(),
            move || Box::new(surface.clone()),
        )]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().absolute_fill())
    }

    fn overlay_opened(&mut self, cx: &mut dyn UpdateContext, id: ElementId, handle: OverlayHandle) {
        self.surface.opened(handle);
        cx.mark_dirty(id);
    }

    fn overlay_closed(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: OverlayHandle,
        _content: Box<dyn WidgetAny>,
    ) {
        if self.surface.closed(handle) {
            // ...and back the other way, so a caller reading the flag sees the dialog
            // close. Two sources of truth, so both directions need saying out loud.
            if let Some(presence) = &self.presence {
                crate::text_input::write_flag(presence, false);
            }
            cx.mark_dirty(id);
        }
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

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
// Tabs
// ---------------------------------------------------------------------------

/// A tabbed navigation: a selectable bar plus the panel for the active tab.
///
/// Selection lives in an `Arc<AtomicUsize>` so a tab button's `on_click` can flip
/// it; `update` notices the change and re-reconciles, mounting the new panel and
/// (because the bar is keyed by selection) re-rendering the highlight. The bar
/// and panel are keyed by the active index, so switching tabs unmounts the old
/// pair and mounts a fresh one rather than mutating in place.
#[derive(Clone)]
pub struct Tabs {
    selection: Arc<AtomicUsize>,
    last_selected: usize,
    tabs: Vec<(String, Child)>,
}

impl std::fmt::Debug for Tabs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tabs")
            .field("tabs", &self.tabs.len())
            .finish()
    }
}

impl Tabs {
    pub fn new() -> Self {
        // `last_selected` starts impossible so the first frame always reconciles.
        Self {
            selection: Arc::new(AtomicUsize::new(0)),
            last_selected: usize::MAX,
            tabs: Vec::new(),
        }
    }

    /// Adds a tab with `title` and its `body` widget subtree.
    pub fn tab(mut self, title: impl Into<String>, body: Child) -> Self {
        self.tabs.push((title.into(), body));
        self
    }

    /// Shared selection handle; the example can also drive it imperatively.
    pub fn selection(&self) -> Arc<AtomicUsize> {
        self.selection.clone()
    }

    /// Selects tab `index`; the next frame re-reconciles if it changed.
    pub fn set_selected(&mut self, index: usize) {
        self.selection.store(index, Ordering::SeqCst);
    }

    fn current(&self) -> usize {
        let n = self.tabs.len();
        if n == 0 {
            return 0;
        }
        self.selection.load(Ordering::SeqCst).min(n - 1)
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Default for Tabs {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Tabs {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let cur = self.current();
        if cur != self.last_selected {
            cx.mark_children_dirty(id);
            self.last_selected = cur;
        }
    }

    fn children(&self) -> Vec<Child> {
        if self.tabs.is_empty() {
            return Vec::new();
        }
        let cur = self.current();
        let sel = self.selection.clone();

        // The bar is keyed by the current selection so it rebuilds (and
        // re-highlights) whenever the active tab changes.
        let mut bar_stack = Stack::new(Axis::Horizontal).gap(4.0);
        for i in 0..self.tabs.len() {
            let sel = sel.clone();
            let active = i == cur;
            let label = if active {
                format!("▶ {}", self.tabs[i].0)
            } else {
                self.tabs[i].0.clone()
            };
            bar_stack = bar_stack.child(Child::new(
                ChildKey::keyed(0xA11B_0000 + i as u64),
                LayoutRequest::new().height(24.0).grow(1.0),
                move || {
                    let sel = sel.clone();
                    Box::new(
                        Button::new(label.clone()).on_click(move || sel.store(i, Ordering::SeqCst)),
                    )
                },
            ));
        }
        let bar_child = Child::new(
            ChildKey::keyed(0xA11B_4000 + cur as u64),
            LayoutRequest::new().stack(Axis::Horizontal, 4.0),
            move || Box::new(bar_stack.clone()),
        );

        let body = self.tabs[cur].1.clone();
        let panel_child = Child::new(
            ChildKey::keyed(0xA11B_8000 + cur as u64),
            LayoutRequest::new().fill(),
            move || body.build(),
        );

        vec![bar_child, panel_child]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(
            LayoutRequest::new()
                .stack(Axis::Vertical, 6.0)
                .fill()
                .cross_align(Align::Stretch),
        )
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
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

// ---------------------------------------------------------------------------
// Popup
// ---------------------------------------------------------------------------

/// Where a [`Popup`]'s panel sits relative to the control that owns it.
///
/// This is a [`Popup`]'s only real decision. Everything else about it — asking the
/// host, learning the handle, being dismissed from outside, falling back to an
/// in-tree layer when the host refuses — is [`OverlayHost`] and was already
/// there. What was missing was geometry: a popup whose rect *is* its own box covers
/// the control, which is right for a menu pinned to a toolbar button and wrong for
/// every dropdown, because a combobox's list belongs *below* its field.
///
/// Kept as three cases rather than a general rect because a caller that can name an
/// arbitrary rect will eventually name one that puts the panel off-screen, and
/// "below the thing I own" is the property that keeps a dropdown attached to its
/// field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PopupPlacement {
    /// Cover the control's own box. The original behaviour, and the default, so
    /// every existing `Popup` is unchanged.
    Fill,
    /// Directly below the control, as wide as it, `gap` pixels of daylight between.
    ///
    /// The height is explicit because the host lays content out *against* the
    /// requested rect and clips rather than growing — so it is a real bound and not
    /// a hint. A caller with a token for it should pass it; a combobox has
    /// `ComboboxTokens::panel_max_height`, which v1 declared and never read.
    Below {
        gap: f64,
        height: f64,
    },
    /// Directly above the control, for one near the bottom of the window.
    ///
    /// Not chosen automatically: "does it fit below?" needs the viewport and a
    /// content height this widget does not have, and a control that silently flips
    /// sides between frames is worse than one that is consistently underneath.
    Above {
        gap: f64,
        height: f64,
    },
}

impl Default for PopupPlacement {
    fn default() -> Self {
        PopupPlacement::Fill
    }
}

/// The rect a placement asks the host for, given the anchor's box.
///
/// Pure geometry, public and separate from the widget so it can be tested without a
/// host, and so a caller that wants to place a panel itself can ask the same question
/// the widget would have answered.
pub fn popup_placement_rect(placement: PopupPlacement, anchor: Rect) -> Rect {
    match placement {
        PopupPlacement::Fill => anchor,
        PopupPlacement::Below { gap, height } => Rect {
            x0: anchor.x0,
            y0: anchor.y1 + gap,
            x1: anchor.x1,
            y1: anchor.y1 + gap + height,
        },
        PopupPlacement::Above { gap, height } => Rect {
            x0: anchor.x0,
            y0: anchor.y0 - gap - height,
            x1: anchor.x1,
            y1: anchor.y0 - gap,
        },
    }
}

/// A transient, non-modal layer anchored above its siblings.
///
/// [`Dialog`] is the modal sibling: it carries a titled card and owns the
/// keyboard. `Popup` is the lighter primitive for menus, tooltips and inline
/// pickers — a togglable layer whose content fills the control's box, with no
/// scrim of its own. It is non-modal, so a click that lands beside it reaches
/// whatever is underneath, and the host dismisses it on that click. Like every
/// overlay composite it contributes nothing while hidden.
#[derive(Clone)]
pub struct Popup {
    surface: OverlayHost,
    label: Option<String>,
    content: Option<Child>,
    placement: PopupPlacement,
}

impl std::fmt::Debug for Popup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Popup")
            .field("present", &self.surface.present)
            .field("label", &self.label)
            .finish()
    }
}

impl Popup {
    pub fn new() -> Self {
        Self {
            surface: OverlayHost::new(false),
            label: None,
            content: None,
            placement: PopupPlacement::default(),
        }
    }

    /// Where the panel sits relative to this control.
    ///
    /// [`PopupPlacement::Fill`] by default, which is to say: unchanged. A dropdown
    /// wants [`PopupPlacement::Below`].
    pub fn placement(mut self, placement: PopupPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// The placement, for a caller that reads it back.
    pub fn current_placement(&self) -> PopupPlacement {
        self.placement
    }

    /// Sets the popup content (does not show it; call [`Popup::set_present`]).
    pub fn content(mut self, child: Child) -> Self {
        self.content = Some(child);
        self
    }

    /// Sets the content with an explicit key and layout.
    pub fn content_with(
        mut self,
        key: impl Into<ChildKey>,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.content = Some(Child::new(key, layout, build));
        self
    }

    /// A label published to assistive technology while the popup is shown.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Shows or hides the popup; the next frame reconciles.
    pub fn set_present(&mut self, present: bool) {
        self.surface.set_present(present);
    }

    /// Hides the popup and drops its content.
    pub fn dismiss(&mut self) {
        self.content = None;
        self.surface.set_present(false);
    }

    /// Whether the popup is up.
    pub fn is_present(&self) -> bool {
        self.surface.is_shown()
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        if self.surface.is_shown() {
            SemanticsHint {
                role: SemanticsRole::Group,
                label: self.label.clone(),
                state: Default::default(),
            }
        } else {
            SemanticsHint::decorative()
        }
    }
}

impl Default for Popup {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Popup {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let content = self.content.clone();
        // A popup is anchored to the control that owns it, so unlike a dialog it
        // takes its own box as the rect the content is laid out against — subject to
        // the placement, which is the one thing that decides whether the panel covers
        // the control or hangs off it.
        let placement = self.placement;
        self.surface.reconcile(cx, id, move |bounds, _viewport| {
            let content = content.clone()?;
            Some(OverlayRequest::new(
                OverlayKind::Popup,
                false,
                popup_placement_rect(placement, bounds),
                move |_handle| content.build(),
            ))
        });
    }

    fn children(&self) -> Vec<Child> {
        if !self.surface.takes_children_in_tree() {
            return Vec::new();
        }
        let content = match &self.content {
            Some(content) => content.clone(),
            None => return Vec::new(),
        };
        // No host overlay: fall back to an in-tree layer, as before.
        let surface = Overlay::new().content_with(
            ChildKey::index(0),
            LayoutRequest::new().fill(),
            move || content.build(),
        );
        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new().absolute_fill(),
            move || Box::new(surface.clone()),
        )]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().absolute_fill())
    }

    fn overlay_opened(&mut self, cx: &mut dyn UpdateContext, id: ElementId, handle: OverlayHandle) {
        self.surface.opened(handle);
        cx.mark_dirty(id);
    }

    fn overlay_closed(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: OverlayHandle,
        _content: Box<dyn WidgetAny>,
    ) {
        if self.surface.closed(handle) {
            cx.mark_dirty(id);
        }
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// BottomSheet
// ---------------------------------------------------------------------------

/// A modal sheet anchored to the bottom edge, spanning the width.
///
/// Structurally a [`Dialog`] that is pinned bottom instead of centred: a
/// bottom-aligned column over the whole window, holding a sheet with the title
/// and body. `height` sets the sheet's height; it stretches to the full width.
/// Being modal it owns the keyboard while it is up, and escapes its parent's
/// clip because the column is a host overlay.
#[derive(Clone)]
pub struct BottomSheet {
    surface: OverlayHost,
    title: String,
    height: f32,
    body: Option<Child>,
}

impl std::fmt::Debug for BottomSheet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BottomSheet")
            .field("present", &self.surface.present)
            .field("height", &self.height)
            .finish()
    }
}

impl BottomSheet {
    pub fn new() -> Self {
        Self {
            surface: OverlayHost::new(false),
            title: String::new(),
            height: 200.0,
            body: None,
        }
    }

    /// The text shown at the top of the sheet.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// The sheet's height in logical pixels.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// The sheet body (any widget subtree).
    pub fn body(mut self, body: Child) -> Self {
        self.body = Some(body);
        self
    }

    /// The sheet body built lazily with an explicit key and layout.
    pub fn body_with(
        mut self,
        key: impl Into<ChildKey>,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.body = Some(Child::new(key, layout, build));
        self
    }

    /// Shows or hides the sheet; the next frame reconciles.
    pub fn set_present(&mut self, present: bool) {
        self.surface.set_present(present);
    }

    /// Whether the sheet is up.
    pub fn is_present(&self) -> bool {
        self.surface.is_shown()
    }

    /// The bottom-aligned column: a fixed-height sheet (full width) pinned to
    /// the bottom edge of the box it is given.
    fn column(&self) -> Stack {
        let title = self.title.clone();
        let height = self.height;
        let body = self.body.clone().unwrap_or_else(empty_body);
        let sheet = Stack::new(Axis::Vertical)
            .gap(8.0)
            .child(Child::new(
                ChildKey::index(0),
                LayoutRequest::new(),
                move || Box::new(Text::new(title.clone())),
            ))
            .child(body);
        let sheet_child = Child::new(
            ChildKey::index(0),
            LayoutRequest::new().height(height),
            move || Box::new(sheet.clone()),
        );
        Stack::new(Axis::Vertical)
            .align(Align::End, Align::Stretch)
            .child(sheet_child)
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        if self.surface.is_shown() {
            SemanticsHint::labeled(SemanticsRole::Dialog, self.title.clone())
        } else {
            SemanticsHint::decorative()
        }
    }
}

impl Default for BottomSheet {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for BottomSheet {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let column = self.column();
        // A sheet is pinned to the bottom of the *window*, not of the box it was
        // declared in, so it is sized from the host viewport.
        self.surface.reconcile(cx, id, move |_bounds, viewport| {
            Some(OverlayRequest::new(
                OverlayKind::BottomSheet,
                true,
                viewport,
                move |_handle| Box::new(column),
            ))
        });
    }

    fn children(&self) -> Vec<Child> {
        if !self.surface.takes_children_in_tree() {
            return Vec::new();
        }
        let column = self.column();
        // No host overlay: fall back to an in-tree layer, as before.
        let surface = Overlay::new().content_with(
            ChildKey::index(0),
            LayoutRequest::new().fill(),
            move || Box::new(column.clone()),
        );
        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new().absolute_fill(),
            move || Box::new(surface.clone()),
        )]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().absolute_fill())
    }

    fn overlay_opened(&mut self, cx: &mut dyn UpdateContext, id: ElementId, handle: OverlayHandle) {
        self.surface.opened(handle);
        cx.mark_dirty(id);
    }

    fn overlay_closed(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: OverlayHandle,
        _content: Box<dyn WidgetAny>,
    ) {
        if self.surface.closed(handle) {
            cx.mark_dirty(id);
        }
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floom_core::{Axis, Position};

    #[test]
    fn stack_reports_its_children_and_axis() {
        let stack = Stack::new(Axis::Vertical)
            .gap(4.0)
            .push(|| Box::new(Text::new("a")))
            .push(|| Box::new(Text::new("b")));
        let children = stack.children();
        assert_eq!(children.len(), 2);
        // Keys default to position, so re-keys survive reordering of slots.
        assert_eq!(children[0].key, ChildKey::index(0));
        assert_eq!(children[1].key, ChildKey::index(1));
        let req = stack.layout_request().expect("stack declares its axis");
        assert_eq!(req.axis, Some(Axis::Vertical));
        assert_eq!(req.gap, Some(4.0));
    }

    #[test]
    fn stack_keyed_children_keep_stable_ids() {
        let stack = Stack::new(Axis::Horizontal)
            .push_keyed(7, || Box::new(Text::new("x")))
            .push_keyed(9, || Box::new(Text::new("y")));
        let children = stack.children();
        assert_eq!(children[0].key, ChildKey::keyed(7));
        assert_eq!(children[1].key, ChildKey::keyed(9));
    }

    #[test]
    fn list_mirrors_items_into_keyed_rows() {
        let list = List::<u32>::new(
            Axis::Vertical,
            vec![10, 20, 30],
            |item: &u32| *item as u64,
            |item: u32| Box::new(Text::new(item.to_string())),
        );
        let children = list.children();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].key, ChildKey::keyed(10));
        assert_eq!(children[2].key, ChildKey::keyed(30));
    }

    #[test]
    fn conditional_shows_only_when_true() {
        let child = Child::new(ChildKey::index(0), LayoutRequest::new(), || {
            Box::new(Text::new("x"))
        });
        assert_eq!(
            Conditional::new(true).show(child.clone()).children().len(),
            1
        );
        assert_eq!(Conditional::new(false).show(child).children().len(), 0);
    }

    #[test]
    fn overlay_is_absolute_and_toggles() {
        let overlay =
            Overlay::new().content(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                Box::new(Text::new("x"))
            }));
        assert_eq!(
            overlay.layout_request().unwrap().position,
            Some(Position::Absolute)
        );
        assert_eq!(overlay.children().len(), 1);
        assert_eq!(Overlay::new().children().len(), 0);
    }

    #[test]
    fn dialog_is_empty_when_hidden_and_present_when_shown() {
        let dialog = Dialog::new().title("Hi").body(Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            || Box::new(Text::new("body")),
        ));
        assert_eq!(dialog.children().len(), 0, "hidden dialog mounts nothing");
        let mut shown = dialog;
        shown.set_present(true);
        assert_eq!(
            shown.children().len(),
            1,
            "present dialog mounts its surface"
        );
    }

    #[test]
    fn tabs_render_bar_and_panel() {
        let tabs = Tabs::new()
            .tab(
                "One",
                Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                    Box::new(Text::new("p1"))
                }),
            )
            .tab(
                "Two",
                Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                    Box::new(Text::new("p2"))
                }),
            );
        let children = tabs.children();
        assert_eq!(children.len(), 2, "bar + active panel");
        let req = tabs
            .layout_request()
            .expect("tabs declare a vertical stack");
        assert_eq!(req.axis, Some(Axis::Vertical));
    }

    #[test]
    fn grid_chunks_items_into_rows() {
        let grid = Grid::<u32>::new(
            2,
            vec![1, 2, 3, 4, 5],
            |item: &u32| *item as u64,
            |item: u32| Box::new(Text::new(item.to_string())),
        );
        let rows = grid.children();
        assert_eq!(rows.len(), 3, "5 items over 2 columns => 3 rows");
        let req = grid.layout_request().expect("grid stacks rows vertically");
        assert_eq!(req.axis, Some(Axis::Vertical));
    }

    #[test]
    fn grid_rows_remount_only_when_membership_changes() {
        let mut grid = Grid::<u32>::new(
            2,
            vec![1, 2, 3, 4],
            |item: &u32| *item as u64,
            |item: u32| Box::new(Text::new(item.to_string())),
        );
        let before: Vec<_> = grid.children().iter().map(|c| c.key).collect();
        // Append a full new row: earlier rows keep their fingerprints.
        grid.set_items(vec![1, 2, 3, 4, 5, 6]);
        let after: Vec<_> = grid.children().iter().map(|c| c.key).collect();
        assert_eq!(before[..2], after[..2], "untouched rows keep their keys");
        assert_eq!(after.len(), 3);
    }

    #[test]
    fn popup_is_empty_until_shown() {
        let popup =
            Popup::new().content(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                Box::new(Text::new("menu"))
            }));
        assert_eq!(popup.children().len(), 0, "hidden popup mounts nothing");
        let mut shown = popup;
        shown.set_present(true);
        assert_eq!(shown.children().len(), 1, "shown popup mounts its layer");
        assert_eq!(
            shown.layout_request().unwrap().position,
            Some(Position::Absolute)
        );
    }

    #[test]
    fn bottom_sheet_mounts_a_bottom_aligned_surface() {
        let sheet = BottomSheet::new()
            .title("Share")
            .height(180.0)
            .body(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                Box::new(Text::new("body"))
            }));
        assert_eq!(sheet.children().len(), 0, "hidden sheet mounts nothing");
        let mut shown = sheet;
        shown.set_present(true);
        assert_eq!(shown.children().len(), 1, "shown sheet mounts its layer");
    }
}

// ---------------------------------------------------------------------------
// Accordion
// ---------------------------------------------------------------------------

/// Builds one panel's expand/collapse indicator, told which way the panel is.
///
/// A closure over the flag rather than two separate widgets (`collapsed` /
/// `expanded`) because the caller cannot know the state when it declares the
/// panel — that is the accordion's answer, and it changes after mount.
pub type AccordionIndicator = Arc<dyn Fn(bool) -> Box<dyn WidgetAny> + Send + Sync + 'static>;

/// Resolved, theme-agnostic style for an [`Accordion`].
///
/// Like every 2.0 widget this takes explicit values: deciding *which* palette
/// and *which* metrics is the design system's job, so a kit resolves them once
/// (`floom-core-theme` -> `MdExpansionPanels::resolve`) and hands the result here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AccordionStyle {
    /// Height of each header row, in logical px.
    pub header_height: f32,
    /// Inset between the header's outline and its content, horizontally.
    pub header_padding: f32,
    /// Inset between the revealed content's edge and its own content.
    pub content_padding: f32,
    /// Extra horizontal indent the revealed content is pushed by, so it reads as
    /// belonging to the panel rather than to the page.
    pub content_indent: f32,
    /// Gap between stacked panels.
    pub gap: f32,
    /// Header title colour.
    pub title_color: Color,
    /// Header title size, in logical px.
    pub title_size: f32,
    /// Outline width drawn just inside each header's edge. `0.0` draws none.
    pub border_width: f32,
    /// Outline colour.
    pub border_color: Color,
    /// Corner radius of the header's outline.
    pub radius: f32,
}

impl Default for AccordionStyle {
    fn default() -> Self {
        Self {
            header_height: 48.0,
            header_padding: 16.0,
            content_padding: 16.0,
            content_indent: 0.0,
            gap: 8.0,
            title_color: Color::BLACK,
            title_size: 14.0,
            border_width: 1.0,
            border_color: Color::rgba(0, 0, 0, 32),
            radius: 12.0,
        }
    }
}

/// One panel of an [`Accordion`].
struct AccordionPanel {
    title: String,
    indicator: AccordionIndicator,
    content: ChildBuilder,
}

/// A stack of panels, each a clickable header that reveals its content.
///
/// This is the **stateful** shape the floem-ui kits all share: a value that
/// changes after mount, a subtree that mounts and unmounts with it, and — the
/// part that is easy to get wrong — a header that must *keep its identity*
/// across the toggle, so keyboard focus is not thrown away by the very act of
/// activating it. All three kits ship this component and their copies differ
/// only in tokens and glyph names, so it is promoted here once.
///
/// ## Where the state lives, and why it is shared
///
/// The click lands on a [`Pressable`] deep inside a header subtree, while the
/// answer — which panels are open — is needed by that panel's body, by its
/// indicator, and in single-panel mode by every *other* panel. None of those
/// owns the accordion's node. So the open set is a shared
/// `Arc<Mutex<Vec<bool>>>`, exactly as [`Tabs`] shares its selection, and every
/// widget that cares reads it in its own `update` and marks itself. The host
/// visits every node each frame, so no notification has to travel.
///
/// ## What this cannot animate
///
/// The reveal is a mount/unmount, not a height transition. v1 animated
/// `height: 0 -> computed` (`md-expand_transition`); 2.0's implicit layer
/// cannot, because `run_layout` only runs when layout is dirty and no
/// compositor channel carries a size. [`Accordion::content_transition`] is
/// therefore compositor-only — a fade, a grow or a slide over the revealed
/// subtree — while the box appears at its final height. See
/// `docs/2.0/MOTION-GAP.md` (B4, the "size" entry under "not done").
pub struct Accordion {
    panels: Vec<AccordionPanel>,
    style: AccordionStyle,
    /// Whether more than one panel may be open at once.
    multiple: bool,
    open: Arc<Mutex<Vec<bool>>>,
    on_change: Option<Arc<dyn Fn(usize, bool) + Send + Sync>>,
    content_transition: Option<PresenceTransition>,
}

impl std::fmt::Debug for Accordion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Accordion")
            .field("panels", &self.panels.len())
            .field("multiple", &self.multiple)
            .field("open", &self.open_state())
            .finish_non_exhaustive()
    }
}

impl Accordion {
    pub fn new() -> Self {
        Self {
            panels: Vec::new(),
            style: AccordionStyle::default(),
            multiple: false,
            open: Arc::new(Mutex::new(Vec::new())),
            on_change: None,
            content_transition: None,
        }
    }

    /// Adds a panel.
    ///
    /// `indicator` is handed the panel's current state and returns the widget
    /// that shows it. `content` builds the body, which is mounted only while the
    /// panel is open (and while its exit transition is still playing).
    pub fn panel(
        mut self,
        title: impl Into<String>,
        indicator: impl Fn(bool) -> Box<dyn WidgetAny> + Send + Sync + 'static,
        content: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        let index = self.panels.len();
        self.lock().resize(index + 1, false);
        self.panels.push(AccordionPanel {
            title: title.into(),
            indicator: Arc::new(indicator),
            content: Arc::new(content),
        });
        self
    }

    /// Allow more than one panel open at a time. Off by default, which is the
    /// accordion rule: opening one closes the others.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Called with `(panel_index, expanded)` whenever a panel's state changes
    /// from a click.
    ///
    /// `Send + Sync` is required, not incidental: the click lands on a
    /// [`Pressable`] built inside a `Child` builder closure, and the host may
    /// hand that closure between threads.
    pub fn on_change(mut self, callback: impl Fn(usize, bool) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// The resolved style. See [`AccordionStyle`].
    pub fn style(mut self, style: AccordionStyle) -> Self {
        self.style = style;
        self
    }

    /// A transition played by a panel's content as it appears and disappears.
    ///
    /// Optional and compositor-only — see the type's note on what cannot be
    /// animated. Left unset, the content mounts and unmounts instantly, which is
    /// what v1's `dyn_view` did.
    pub fn content_transition(mut self, transition: PresenceTransition) -> Self {
        self.content_transition = Some(transition);
        self
    }

    /// Which panels are open right now.
    pub fn open_state(&self) -> Vec<bool> {
        self.lock().clone()
    }

    /// Whether panel `index` is open.
    pub fn is_open(&self, index: usize) -> bool {
        self.lock().get(index).copied().unwrap_or(false)
    }

    /// Opens or closes panel `index` without going through a click.
    ///
    /// The imperative door, the twin of [`Tabs::set_selected`]. It does **not**
    /// fire [`Accordion::on_change`]: that callback reports what a *person* did,
    /// and a programmatic caller already knows. It does apply the
    /// single/multiple rule, so opening a panel in accordion mode closes the
    /// others.
    pub fn set_open(&mut self, index: usize, open: bool) {
        let mut guard = self.lock();
        if guard.len() <= index {
            guard.resize(index + 1, false);
        }
        if !self.multiple {
            for slot in guard.iter_mut() {
                *slot = false;
            }
        }
        guard[index] = open;
    }

    pub fn panel_count(&self) -> usize {
        self.panels.len()
    }

    pub fn titles(&self) -> Vec<&str> {
        self.panels.iter().map(|p| p.title.as_str()).collect()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<bool>> {
        // A poisoned lock means some other holder panicked mid-toggle. The data
        // is a `Vec<bool>` only ever written whole, so it is still coherent, and
        // refusing to draw an accordion because an unrelated callback panicked
        // would be the worse failure.
        self.open.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Flips panel `index` and returns its new state, applying the
    /// single/multiple rule.
    fn toggle(open: &Mutex<Vec<bool>>, index: usize, multiple: bool) -> bool {
        let mut guard = open.lock().unwrap_or_else(|e| e.into_inner());
        let next = !guard.get(index).copied().unwrap_or(false);
        if !multiple {
            for slot in guard.iter_mut() {
                *slot = false;
            }
        }
        if let Some(slot) = guard.get_mut(index) {
            *slot = next;
        }
        next
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Default for Accordion {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Accordion {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    /// Deliberately empty.
    ///
    /// Nothing changed *here*: the accordion still owns the same panel slots, and
    /// what moved is the shared open set. Every panel reads that set in its own
    /// `update` and reports the change itself, so marking dirty here too would
    /// be a second report of a change already reported — the same reasoning (and
    /// deliberately the same wording) as [`Crossfade::update`].
    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}

    fn children(&self) -> Vec<Child> {
        let mut out = Vec::with_capacity(self.panels.len());
        for (index, panel) in self.panels.iter().enumerate() {
            let open = self.open.clone();
            let on_change = self.on_change.clone();
            let multiple = self.multiple;
            let toggle = Arc::new(move || {
                let next = Self::toggle(&open, index, multiple);
                if let Some(callback) = &on_change {
                    callback(index, next);
                }
            }) as Arc<dyn Fn() + Send + Sync>;

            let mut body = Conditional::new(true);
            // Seeded closed, and *seeded* rather than left at the constructor's
            // "not yet seen": see `Conditional::sync_when`.
            body.sync_when(false);
            let content = panel.content.clone();
            body = body.show_with(
                ChildKey::index(1),
                LayoutRequest::new().margin(content_insets(self.style)),
                move || content(),
            );
            if let Some(transition) = self.content_transition {
                body = body.presence(transition);
            }

            let leaf = AccordionPanelLeaf {
                index,
                open: self.open.clone(),
                last_open: false,
                style: self.style,
                header: header_child(self, index, panel, toggle),
                body,
            };
            out.push(Child::new(
                ChildKey::index(index),
                LayoutRequest::new(),
                move || Box::new(leaf.clone()),
            ));
        }
        out
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().stack(Axis::Vertical, self.style.gap))
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

/// The inset a revealed panel's body sits inside its panel box.
///
/// v1 wrote this as `padding_horiz(content_padding + indent)` and
/// `padding_vert(content_padding)` on the box holding the body. 2.0's `Container`
/// is a leaf, so the inset is a *margin on the body's slot* instead — see
/// `LayoutRequest::margin`.
fn content_insets(style: AccordionStyle) -> Insets {
    let horizontal = f64::from(style.content_padding + style.content_indent);
    let vertical = f64::from(style.content_padding);
    Insets {
        left: horizontal,
        right: horizontal,
        top: vertical,
        bottom: vertical,
    }
}

/// The header slot: the whole of the panel's top row, clickable.
///
/// Keyed by [`ChildKey::index`] on purpose. The row has to survive the toggle —
/// a rebuilt header is a new node, so the keyboard focus [`Pressable`] requests
/// on the press that *causes* the toggle would be thrown away by activating it.
/// Everything inside that has to change therefore has to be a widget that
/// re-declares its own children, which is what [`AccordionIndicatorSlot`] is.
fn header_child(
    accordion: &Accordion,
    index: usize,
    panel: &AccordionPanel,
    toggle: Arc<dyn Fn() + Send + Sync + 'static>,
) -> Child {
    let style = accordion.style;
    let open = accordion.open.clone();
    let title = panel.title.clone();
    let indicator = panel.indicator.clone();
    Child::new(
        ChildKey::index(0),
        LayoutRequest::new().height(style.header_height).fill(),
        move || {
            let open_flag = open.clone();
            let row = Stack::new(Axis::Horizontal)
                .align(Align::Start, Align::Center)
                .push({
                    let title = title.clone();
                    move || {
                        Box::new(
                            Text::new(title.clone())
                                .size(style.title_size)
                                .color(style.title_color),
                        )
                    }
                })
                // v1 asked for `justify_between`, which 2.0's single `Align`
                // cannot say; a flexible spacer is the flex spelling of "push
                // these apart".
                .push(|| Box::new(crate::primitives::Spacer::flexible()))
                .push({
                    let indicator = indicator.clone();
                    let open_flag = open_flag.clone();
                    move || {
                        Box::new(AccordionIndicatorSlot {
                            open: open_flag.clone(),
                            index,
                            indicator: indicator.clone(),
                            last_open: false,
                        })
                    }
                });

            Box::new(
                Pressable::new(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new().fill(),
                    move || Box::new(row.clone()),
                ))
                .label(title.clone())
                .expanded_from(move || is_open(&open_flag, index))
                .on_click({
                    let toggle = toggle.clone();
                    move || toggle()
                }),
            )
        },
    )
}

/// One panel of an [`Accordion`]: the header plus the body its open flag mounts
/// and unmounts.
///
/// Stable for the panel's whole life, which is what keeps the header's focus.
/// Everything it shows is derived from the shared open set on every `update`.
///
/// It also paints the panel's own outline, which is the one place this port
/// departs from v1: v1 outlined the *header row* with
/// `.border(1.0).border_color(..)`, and 2.0 cannot reproduce that shape — the
/// box that would have to carry the outline is a `Container`, which is a leaf and
/// so cannot hold the row it would be outlining (see `docs/`). Outlining the
/// panel instead is both what is available and what Material 3's outlined
/// expansion panel is: one box holding the header and, when open, the content.
#[derive(Clone)]
pub(crate) struct AccordionPanelLeaf {
    index: usize,
    open: Arc<Mutex<Vec<bool>>>,
    /// The open state this leaf last acted on.
    last_open: bool,
    style: AccordionStyle,
    header: Child,
    /// The reveal, held as a value rather than mounted as a node of its own so
    /// the presence transition is declared on *this* node — one node instead of a
    /// wrapper, and the transition's `animation_finished` lands here too.
    body: Conditional,
}

impl Widget for AccordionPanelLeaf {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        if is_open(&self.open, self.index) == self.last_open {
            return;
        }
        self.last_open = !self.last_open;
        // `Conditional::update` is what marks this node dirty *and* reconciles
        // its children; passing this node's id is deliberate — the transition is
        // declared here, so the host re-syncs the declaration against it.
        self.body.set_when(self.last_open);
        self.body.update(cx, id);
    }

    fn children(&self) -> Vec<Child> {
        let mut out = vec![self.header.clone()];
        out.extend(self.body.children());
        out
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().stack(Axis::Vertical, 0.0))
    }

    fn animation(&self) -> Option<floom_core::AnimationSpec> {
        self.body.animation()
    }

    fn animation_finished(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.body.animation_finished(cx, id);
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        if self.style.border_width <= 0.0 {
            return;
        }
        let bounds = cx.bounds();
        crate::primitives::stroke_rounded_outline(
            cx,
            bounds,
            f64::from(self.style.radius),
            f64::from(self.style.border_width),
            self.style.border_color,
        );
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(SemanticsHint::decorative(), cx);
    }
}

/// The disclosure indicator of one panel.
///
/// Stable across toggles — that is what lets the header be — but it re-declares
/// its own single child keyed by the open flag, which is what swaps one glyph for
/// the other. A bare [`Child`] could not do this: the host only rebuilds a child
/// whose key *changed*, and everything inside a stable header has a stable key.
#[derive(Clone)]
pub(crate) struct AccordionIndicatorSlot {
    open: Arc<Mutex<Vec<bool>>>,
    index: usize,
    indicator: AccordionIndicator,
    last_open: bool,
}

impl Widget for AccordionIndicatorSlot {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        if is_open(&self.open, self.index) == self.last_open {
            return;
        }
        self.last_open = !self.last_open;
        // Reconcile for the new glyph, and repaint so the row is re-measured
        // around an indicator of a different size.
        cx.mark_children_dirty(id);
        cx.mark_dirty(id);
    }

    fn children(&self) -> Vec<Child> {
        let indicator = self.indicator.clone();
        let open = self.last_open;
        vec![Child::new(
            // The flag is the key, so the host mounts a *different* widget rather
            // than re-using the previous glyph and diffing two builders that
            // both mean "the indicator".
            ChildKey::keyed(INDICATOR_KEY_BASE ^ (open as u64)),
            LayoutRequest::new(),
            move || indicator(open),
        )]
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(SemanticsHint::decorative(), cx);
    }
}

/// Distinct from the positional keys an accordion's slots use, so a keyed
/// indicator can never collide with one of them.
const INDICATOR_KEY_BASE: u64 = 0xACC0_0000_0000_0000;

/// Whether panel `index` of a shared open set is open.
fn is_open(open: &Mutex<Vec<bool>>, index: usize) -> bool {
    open.lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(index)
        .copied()
        .unwrap_or(false)
}

/// How many rows of a [`SelectGroup`] may be selected at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    /// At most one. Choosing a row replaces the selection.
    #[default]
    Single,
    /// Any number. Choosing a row toggles it.
    Multiple,
}

/// The two appearances of a [`SelectGroup`] row.
///
/// A resolved style struct rather than a builder chain, because every row needs
/// both appearances and the choice between them happens in the row's `update`,
/// long after any builder ran.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectGroupStyle {
    /// Gap between rows.
    pub gap: f32,
    /// Row padding: `pad` horizontally, `pad / 2` vertically, as v1 wrote it.
    pub pad: f32,
    pub radius: f32,
    /// The outline's thickness. `0.0` draws none.
    pub border_width: f32,
    /// Unselected row.
    pub background: Color,
    pub foreground: Color,
    pub border_color: Color,
    /// Selected row.
    pub selected_background: Color,
    pub selected_foreground: Color,
    pub selected_border_color: Color,
}

impl Default for SelectGroupStyle {
    fn default() -> Self {
        Self {
            gap: 4.0,
            pad: 16.0,
            radius: 8.0,
            border_width: 1.0,
            background: Color::rgb(0xF5, 0xF5, 0xF5),
            foreground: Color::rgb(0x1C, 0x1B, 0x1F),
            border_color: Color::rgb(0x79, 0x79, 0x7E),
            selected_background: Color::rgb(0xCF, 0xD8, 0xF7),
            selected_foreground: Color::rgb(0x00, 0x2A, 0x77),
            selected_border_color: Color::rgb(0x41, 0x4F, 0x7B),
        }
    }
}

/// One row's content.
struct SelectRow {
    /// What assistive technology calls the row. v1 had no semantics at all for
    /// these rows — see [`SelectGroup`] on why the port adds them.
    label: Option<String>,
    content: ChildBuilder,
}

/// A stack of rows, any number of which can be selected.
///
/// This is the **selectable** shape the floem-ui kits all share, and the third
/// promoted widget for the same reason [`Accordion`] was: `md-item_group`,
/// `cupertino-item_group` and `fluent-item_group` are 209, 208 and 208 lines with
/// **123 of 149** substantive lines identical — only the style struct's field
/// names and the `design_system` string differ. Porting three times would be three
/// chances to get one state machine wrong.
///
/// ## Where the state lives, and why it is shared
///
/// The click lands on a [`Pressable`] inside a row, while the answer — which rows
/// are selected — is needed by *every* row, because selecting one changes the fill
/// and outline of the others as well. No row owns the group's node. So the
/// selection is a shared `Arc<Mutex<Vec<bool>>>`, exactly as [`Accordion`] shares
/// its open set and [`Tabs`] shares its selection, and every row reads it in its
/// own `update` and marks itself. The host visits every node each frame, so no
/// notification has to travel.
///
/// ## Two things this adds that v1 did not have
///
/// * **Semantics.** v1's rows were a `Container` with a click handler and a
///   pointer cursor: invisible to a screen reader, which could neither name a row
///   nor learn which was chosen. A row here is a [`Pressable`] with
///   [`SemanticsRole::Radio`] in single mode and [`SemanticsRole::CheckBox`] in
///   multiple mode, reading its own selected state through
///   [`Pressable::selected_from`]. That is a deliberate addition rather than a
///   translation, and it is called out in the porting document for that reason.
/// * **A selected row reports its state by reader, not by value.** A row is not
///   rebuilt when the selection moves, so a captured value would go stale the
///   instant another row was chosen — and would be the *opposite* of what the
///   user sees.
///
/// ## What this cannot do
///
/// v1 hid the whole group when it had no rows (`apply_if(n == 0, |s| s.hide())`).
/// 2.0 has no `hide()`: [`LayoutRequest`] carries no visibility, and a hidden
/// node and an absent one differ in layout — one takes no space, the other takes
/// whatever the parent gives it. An empty group here is a stack with no children.
pub struct SelectGroup {
    rows: Vec<SelectRow>,
    selected: Arc<Mutex<Vec<bool>>>,
    mode: SelectionMode,
    mandatory: bool,
    style: SelectGroupStyle,
    on_change: Option<Arc<dyn Fn(Vec<usize>) + Send + Sync>>,
}

impl SelectGroup {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            selected: Arc::new(Mutex::new(Vec::new())),
            mode: SelectionMode::Single,
            mandatory: false,
            style: SelectGroupStyle::default(),
            on_change: None,
        }
    }

    /// Append a row.
    pub fn row(
        mut self,
        label: impl Into<String>,
        content: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.rows.push(SelectRow {
            label: Some(label.into()),
            content: Arc::new(content),
        });
        self
    }

    /// Append a row with no accessible name — one whose content speaks for itself.
    pub fn row_unlabelled(
        mut self,
        content: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.rows.push(SelectRow {
            label: None,
            content: Arc::new(content),
        });
        self
    }

    pub fn mode(mut self, mode: SelectionMode) -> Self {
        self.mode = mode;
        self
    }

    /// Several rows may be chosen.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.mode = if multiple {
            SelectionMode::Multiple
        } else {
            SelectionMode::Single
        };
        self
    }

    /// The selection may not be emptied: in single mode the chosen row cannot be
    /// un-chosen, and in multiple mode the last chosen row cannot be un-chosen.
    pub fn mandatory(mut self, mandatory: bool) -> Self {
        self.mandatory = mandatory;
        self
    }

    pub fn style(mut self, style: SelectGroupStyle) -> Self {
        self.style = style;
        self
    }

    /// The style as declared — the token-to-colour translation, readable back.
    pub fn declared_style(&self) -> SelectGroupStyle {
        self.style
    }

    /// Called with the selected indices, in order, after every change.
    pub fn on_change(mut self, callback: impl Fn(Vec<usize>) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// Choose a row before mount.
    pub fn selected(self, indices: impl IntoIterator<Item = usize>) -> Self {
        for index in indices {
            if self
                .selected
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .len()
                <= index
            {
                self.selected
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .resize(index + 1, false);
            }
            self.selected.lock().unwrap_or_else(|e| e.into_inner())[index] = true;
        }
        self
    }

    /// The colour a row's *content* should be drawn in.
    ///
    /// The third component in a row where v1 wrote `.color(fg)` on the box: 2.0's
    /// `Surface` paints a fill, and only the child knows it is text. So the ink is
    /// a question for the caller rather than a property of the decoration, and the
    /// pair is reported together so a caller cannot style a selected row's box and
    /// leave its label in the unselected ink.
    pub fn foreground(&self, selected: bool) -> Color {
        if selected {
            self.style.selected_foreground
        } else {
            self.style.foreground
        }
    }

    /// The selection as indices — the shape a caller reports or compares.
    pub fn selection(&self) -> Vec<usize> {
        is_selected_indices(&self.selected)
    }

    /// Whether one row is chosen, for a caller mirroring the state.
    pub fn is_selected(&self, index: usize) -> bool {
        is_selected(&self.selected, index)
    }

    /// How many rows may be chosen, as declared.
    pub fn selection_mode(&self) -> SelectionMode {
        self.mode
    }

    /// Whether the selection may be emptied, as declared.
    pub fn is_mandatory(&self) -> bool {
        self.mandatory
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        // A `Group` with no label rather than an invented one: v1 had no group
        // name, and an empty string is not a name.
        SemanticsHint {
            role: SemanticsRole::Group,
            label: None,
            state: Default::default(),
        }
    }
}

impl Default for SelectGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for SelectGroup {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    /// Deliberately empty, for the reason [`Accordion::update`] gives: every row
    /// reads the shared selection and reports its own change, so marking dirty here
    /// as well would be a second report of a change already reported.
    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}

    fn children(&self) -> Vec<Child> {
        let mut out = Vec::with_capacity(self.rows.len());
        for (index, row) in self.rows.iter().enumerate() {
            let selected = self.selected.clone();
            let label = row.label.clone();
            let content = row.content.clone();
            let mode = self.mode;
            let mandatory = self.mandatory;
            let style = self.style;
            let on_change = self.on_change.clone();

            let toggle = Arc::new(move || {
                // The rules live in `SelectGroup::choose`, reached through a
                // closure that owns the same four values a row does. A row cannot
                // call `&self`: it is not the group.
                let changed = apply_choose(&selected, index, mode, mandatory);
                if changed {
                    if let Some(callback) = &on_change {
                        callback(is_selected_indices(&selected));
                    }
                }
            }) as Arc<dyn Fn() + Send + Sync + 'static>;

            let leaf = SelectGroupRow {
                index,
                selected: self.selected.clone(),
                last_selected: false,
                mode,
                label,
                style,
                content,
                toggle,
            };
            out.push(Child::new(
                ChildKey::index(index),
                LayoutRequest::new(),
                move || Box::new(leaf.clone()),
            ));
        }
        out
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().stack(Axis::Vertical, self.style.gap))
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

/// One row of a [`SelectGroup`]: the click target, the decorated box, and the
/// child inside it.
#[derive(Clone)]
pub(crate) struct SelectGroupRow {
    index: usize,
    selected: Arc<Mutex<Vec<bool>>>,
    /// The value this row last acted on, so a change is a change rather than a
    /// fresh read every frame.
    last_selected: bool,
    mode: SelectionMode,
    label: Option<String>,
    style: SelectGroupStyle,
    content: ChildBuilder,
    toggle: Arc<dyn Fn() + Send + Sync>,
}

impl Widget for SelectGroupRow {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let now = is_selected(&self.selected, self.index);
        if now == self.last_selected {
            return;
        }
        self.last_selected = now;
        // The fill and the outline are this row's own decoration, so this row is
        // what has to invalidate them. A dirty mark and nothing else: the box does
        // not move, so there is no layout to redo, and `mark_layout_dirty` here
        // would re-lay out the group for a change of colour.
        cx.mark_dirty(id);
    }

    fn children(&self) -> Vec<Child> {
        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new().fill(),
            {
                let index = self.index;
                let selected = self.selected.clone();
                let label = self.label.clone();
                let content = self.content.clone();
                let style = self.style;
                let mode = self.mode;
                let toggle = self.toggle.clone();
                move || {
                    let role = match mode {
                        SelectionMode::Single => SemanticsRole::Radio,
                        SelectionMode::Multiple => SemanticsRole::CheckBox,
                    };
                    let selected_flag = selected.clone();
                    let surface =
                        row_surface(style, is_selected(&selected_flag, index), content.clone());
                    let mut pressable = crate::controls::Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || Box::new(surface.clone()),
                    ))
                    .role(role)
                    .selected_from(move || is_selected(&selected_flag, index));
                    // Cloned, not moved: this closure is replayed, so the label has to survive
                    // being handed to a rebuilt `Pressable`.
                    if let Some(label) = label.clone() {
                        pressable = pressable.label(label);
                    }
                    // The toggle is cloned into the click closure, not moved: this slot is
                    // replayed, so a rebuilt row has to be able to hand out its own handle.
                    let click = toggle.clone();
                    Box::new(pressable.on_click(move || click()))
                }
            },
        )]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().fill())
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(SemanticsHint::decorative(), cx);
    }
}

/// The decorated box of one row, in the appearance its selection calls for.
fn row_surface(
    style: SelectGroupStyle,
    selected: bool,
    content: ChildBuilder,
) -> crate::surface::Surface {
    let (background, border) = if selected {
        (style.selected_background, style.selected_border_color)
    } else {
        (style.background, style.border_color)
    };
    let mut surface = crate::surface::Surface::new()
        .background(background)
        .radius(style.radius)
        // v1: `padding_horiz(pad).padding_vert(pad / 2.0)`
        .padding(Insets {
            left: f64::from(style.pad),
            right: f64::from(style.pad),
            top: f64::from(style.pad) / 2.0,
            bottom: f64::from(style.pad) / 2.0,
        });
    if style.border_width > 0.0 {
        surface = surface.border(style.border_width, border);
    }
    surface.push(move || content())
}

/// `SelectGroup::choose`, as a free function over the shared state.
fn apply_choose(
    selected: &Mutex<Vec<bool>>,
    index: usize,
    mode: SelectionMode,
    mandatory: bool,
) -> bool {
    let mut guard = selected.lock().unwrap_or_else(|e| e.into_inner());
    if guard.len() <= index {
        guard.resize(index + 1, false);
    }
    let was = guard[index];
    match mode {
        SelectionMode::Single => {
            if mandatory && was {
                return false;
            }
            for slot in guard.iter_mut() {
                *slot = false;
            }
            guard[index] = true;
            true
        }
        SelectionMode::Multiple => {
            if was {
                let chosen = guard.iter().filter(|s| **s).count();
                if mandatory && chosen <= 1 {
                    return false;
                }
                guard[index] = false;
            } else {
                guard[index] = true;
            }
            true
        }
    }
}

fn is_selected(selected: &Mutex<Vec<bool>>, index: usize) -> bool {
    selected
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(index)
        .copied()
        .unwrap_or(false)
}

fn is_selected_indices(selected: &Mutex<Vec<bool>>) -> Vec<usize> {
    selected
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .enumerate()
        .filter(|(_, chosen)| **chosen)
        .map(|(index, _)| index)
        .collect()
}

#[cfg(test)]
mod select_group_tests {
    //! The **selectable** reference widget.
    //!
    //! Where `accordion_tests` pins a value that shows and hides a subtree, this
    //! pins one that *restyles* the rows around it: choosing a row changes the fill
    //! and outline of every row, not only the chosen one. The three things that make
    //! that hard are each a test here.
    //!
    //! The tests stand in for the host, for the reason `accordion_tests` gives: the
    //! update pass visits every node every frame and dirtiness is *reported by*
    //! `update()`. So what is checked is that each row reports its own change.

    use floom_accessibility::SemanticsState;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use floom_core::{Edges, Point, PointerEvent, Widget};
    use floom_types::Rect;

    use super::*;
    use crate::primitives::Text;
    use crate::testkit::{AllocatingCreateContext, RecordingUpdateContext, TestEventContext};

    const ROW: Rect = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 300.0,
        y1: 48.0,
    };
    const AT: Point = Point { x: 20.0, y: 20.0 };

    fn three_rows() -> SelectGroup {
        let mut group = SelectGroup::new();
        for label in ["One", "Two", "Three"] {
            group = group.row(label, || Box::new(Text::new("row")));
        }
        group
    }

    /// Mounts row `index` the way the reconciler would.
    fn mount_row(group: &SelectGroup, index: usize) -> SelectGroupRow {
        let child = group.children()[index].clone();
        let mut widget = child.build();
        widget
            .as_any_mut()
            .downcast_mut::<SelectGroupRow>()
            .expect("a row slot mounts a row")
            .clone()
    }

    fn tick(row: &mut SelectGroupRow) -> RecordingUpdateContext {
        let mut create = AllocatingCreateContext::new();
        let id = row.create(&mut create);
        let mut cx = RecordingUpdateContext::new();
        row.update(&mut cx, id);
        cx
    }

    /// Clicks row `index` through the real dispatch path: a press and a release,
    /// both consumed by that row's own `Pressable`.
    fn click_row(group: &SelectGroup, index: usize) {
        let mut mounted = group.children()[index].build().children().remove(0).build();
        let pressable = mounted
            .as_any_mut()
            .downcast_mut::<Pressable>()
            .expect("a row's slot mounts a pressable");
        let mut pressable = std::mem::replace(
            pressable,
            Pressable::new(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                Box::new(Text::new(""))
            })),
        );
        let mut create = AllocatingCreateContext::new();
        let id = pressable.create(&mut create);
        for phase in [PointerEvent::primary_down(AT), PointerEvent::primary_up(AT)] {
            let mut cx = TestEventContext::new(id, ROW, phase.into());
            pressable.event(&mut cx, id);
        }
    }

    // --- structure ---------------------------------------------------------

    #[test]
    fn every_declared_row_gets_one_slot() {
        let group = three_rows();
        let children = group.children();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].key, ChildKey::index(0));
        assert_eq!(children[2].key, ChildKey::index(2));
    }

    #[test]
    fn the_stack_declares_its_axis_and_the_token_gap() {
        let group = three_rows().style(SelectGroupStyle {
            gap: 7.0,
            ..SelectGroupStyle::default()
        });
        let req = group.layout_request().expect("the group is a stack");
        assert_eq!(req.axis, Some(Axis::Vertical));
        assert_eq!(req.gap, Some(7.0));
    }

    /// The mutation this catches: an empty group whose `children()` is wrong would
    /// mount nothing, and v1's `apply_if(n == 0, |s| s.hide())` hid the *node*.
    /// 2.0 has no `hide()` — see `SelectGroup`'s doc — so an empty group is a
    /// stack with no children, and this says so rather than leaving it implied.
    #[test]
    fn an_empty_group_is_a_stack_with_no_rows() {
        let group = SelectGroup::new();
        assert!(group.children().is_empty());
        assert_eq!(
            group.layout_request().and_then(|r| r.axis),
            Some(Axis::Vertical),
            "and it is still a stack"
        );
    }

    // --- the click rules ---------------------------------------------------

    #[test]
    fn clicking_a_row_selects_it() {
        let group = three_rows();
        assert!(
            group.selection().is_empty(),
            "nothing is chosen to begin with"
        );
        click_row(&group, 1);
        assert_eq!(group.selection(), vec![1]);
        assert!(group.is_selected(1));
        assert!(!group.is_selected(0));
    }

    /// The control for the above, and the reason `apply_choose` clears the rest: if
    /// choosing did *not* clear, the two tests would agree with each other and
    /// nothing would notice.
    #[test]
    fn choosing_a_second_row_replaces_the_first_in_single_mode() {
        let group = three_rows();
        click_row(&group, 0);
        click_row(&group, 2);
        assert_eq!(group.selection(), vec![2], "not [0, 2]");
    }

    #[test]
    fn multiple_mode_accumulates_and_toggles() {
        let group = three_rows().multiple(true);
        click_row(&group, 0);
        click_row(&group, 2);
        assert_eq!(group.selection(), vec![0, 2]);
        click_row(&group, 0);
        assert_eq!(
            group.selection(),
            vec![2],
            "clicking a chosen row un-chooses it"
        );
    }

    /// v1's single-mode mandatory rule: the chosen row cannot be un-chosen.
    #[test]
    fn mandatory_single_mode_refuses_to_empty_the_selection() {
        let group = three_rows().mandatory(true);
        click_row(&group, 1);
        assert_eq!(group.selection(), vec![1]);
        click_row(&group, 1);
        assert_eq!(
            group.selection(),
            vec![1],
            "clicking the chosen row again is a no-op"
        );
        click_row(&group, 2);
        assert_eq!(group.selection(), vec![2], "but another row still takes it");
    }

    /// v1's multiple-mode mandatory rule, which is narrower than the name suggests
    /// and narrower than this test first asserted: `mandatory` refuses to remove a
    /// row only when it is the *only* chosen one (`cur.len() == 1` in v1). With two
    /// chosen, clicking either one removes it — mandatory keeps the set non-empty,
    /// it does not freeze it.
    ///
    /// Recorded because getting this backwards is invisible until a user tries to
    /// deselect one of two chosen rows and nothing happens.
    #[test]
    fn mandatory_multiple_mode_keeps_the_selection_non_empty() {
        let group = three_rows().multiple(true).mandatory(true);
        click_row(&group, 0);
        click_row(&group, 1);
        assert_eq!(group.selection(), vec![0, 1]);

        click_row(&group, 1);
        assert_eq!(
            group.selection(),
            vec![0],
            "one of two may go: mandatory is not a freeze"
        );

        click_row(&group, 0);
        assert_eq!(group.selection(), vec![0], "and the last one may not");

        click_row(&group, 2);
        assert_eq!(group.selection(), vec![0, 2], "adding still works");
    }

    #[test]
    fn a_row_chosen_before_mount_starts_selected() {
        let group = three_rows().selected([1]);
        assert_eq!(group.selection(), vec![1]);
        // And it is *reported*, not merely stored.
        let mut row = mount_row(&group, 1);
        let cx = tick(&mut row);
        assert_eq!(
            row.last_selected, true,
            "the row noticed it was already chosen"
        );
        let _ = cx;
    }

    // --- the report --------------------------------------------------------

    #[test]
    fn a_change_reports_the_new_selection() {
        let seen: Arc<Mutex<Vec<Vec<usize>>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        let group = three_rows().multiple(true).on_change(move |next| {
            sink.lock().unwrap().push(next);
        });
        click_row(&group, 0);
        click_row(&group, 2);
        assert_eq!(
            *seen.lock().unwrap(),
            vec![vec![0], vec![0, 2]],
            "one report per change, in order"
        );
    }

    /// A refused change must not report: v1 called its callback only when it had
    /// actually changed something, and a consumer counting notifications would
    /// otherwise see a phantom event for a no-op click.
    #[test]
    fn a_refused_change_reports_nothing() {
        let count = Arc::new(AtomicUsize::new(0));
        let sink = count.clone();
        let group = three_rows().mandatory(true).on_change(move |_| {
            sink.fetch_add(1, Ordering::SeqCst);
        });
        click_row(&group, 1);
        assert_eq!(count.load(Ordering::SeqCst), 1);
        click_row(&group, 1);
        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "the second click was refused, so nothing was reported"
        );
    }

    // --- the shared state, which is the whole design -----------------------

    /// The load-bearing claim. A click lands inside one row, but the answer is
    /// needed by every row; if the state were per-row, choosing row 1 would leave
    /// row 0 believing it was still chosen.
    #[test]
    fn choosing_one_row_is_visible_to_the_others() {
        let group = three_rows();
        click_row(&group, 1);
        for index in 0..3 {
            assert_eq!(
                group.is_selected(index),
                index == 1,
                "row {index} reads the shared selection"
            );
        }
    }

    /// The mutation this catches: a row whose `update` compares against nothing
    /// and so never notices. Every other test here drives the *group's* accessor,
    /// which would keep passing.
    #[test]
    fn a_row_reports_its_own_change_rather_than_being_told() {
        let group = three_rows();
        let mut row = mount_row(&group, 0);
        assert_eq!(row.last_selected, false, "unselected to begin with");

        // A first tick with nothing changed reports nothing.
        let quiet = tick(&mut row);
        assert!(
            quiet.dirty_count() == 0,
            "an unselected row that stays unselected says nothing"
        );

        click_row(&group, 0);
        let loud = tick(&mut row);
        assert!(loud.dirty_count() > 0, "and this row noticed by itself");
        assert_eq!(row.last_selected, true);

        // And settles: a second tick after the change is quiet again.
        let settled = tick(&mut row);
        assert!(
            settled.dirty_count() == 0,
            "a change is reported once, not every frame"
        );
    }

    #[test]
    fn a_row_that_loses_the_selection_reports_too() {
        let group = three_rows().selected([0]);
        let mut row = mount_row(&group, 0);
        tick(&mut row);
        assert_eq!(row.last_selected, true);

        click_row(&group, 1);
        let cx = tick(&mut row);
        assert!(
            cx.dirty_count() > 0,
            "losing the selection is a change worth reporting"
        );
        assert_eq!(row.last_selected, false);
    }

    // --- the two appearances ----------------------------------------------

    /// The decoration each row draws is the other half of "the state is shared":
    /// the row that changed appearance is the row that reports.
    #[test]
    fn a_row_paints_the_selected_appearance_when_selected() {
        let style = SelectGroupStyle::default();
        let content: ChildBuilder = Arc::new(|| Box::new(Text::new("row")));

        let unselected = row_surface(style, false, content.clone());
        let selected = row_surface(style, true, content);

        let fill_of = |surface: &crate::surface::Surface| {
            surface
                .decoration()
                .background_color()
                .expect("a row is filled")
        };
        assert_eq!(
            fill_of(&unselected),
            style.background,
            "the unselected fill"
        );
        assert_eq!(fill_of(&selected), style.selected_background);
        assert_ne!(
            fill_of(&unselected),
            fill_of(&selected),
            "and the two are different, so the change is visible at all"
        );
    }

    #[test]
    fn a_row_outlines_itself_at_the_style_width() {
        let style = SelectGroupStyle {
            border_width: 2.0,
            ..SelectGroupStyle::default()
        };
        let surface = row_surface(style, false, Arc::new(|| Box::new(Text::new("row"))));
        let border = surface
            .decoration()
            .border_spec()
            .expect("v1 wrote `.border(1.0)` unconditionally");
        assert_eq!(border.width(), 2.0);
        assert_eq!(border.color(), style.border_color);
        assert_eq!(border.edges(), Edges::ALL, "v1 outlined all four sides");
    }

    #[test]
    fn a_zero_border_width_draws_none() {
        let style = SelectGroupStyle {
            border_width: 0.0,
            ..SelectGroupStyle::default()
        };
        let surface = row_surface(style, false, Arc::new(|| Box::new(Text::new("row"))));
        assert!(surface.decoration().border_spec().is_none());
    }

    /// v1 wrote `padding_horiz(pad).padding_vert(pad / 2.0)`, which is not a
    /// uniform inset and is the kind of asymmetry a "just use `uniform`" cleanup
    /// would quietly lose.
    #[test]
    fn the_row_padding_is_horizontal_pad_and_vertical_half() {
        let style = SelectGroupStyle {
            pad: 16.0,
            ..SelectGroupStyle::default()
        };
        let surface = row_surface(style, false, Arc::new(|| Box::new(Text::new("row"))));
        let declared = surface.layout_request().expect("a padded surface");
        assert_eq!(
            declared.padding,
            Some(floom_core::Insets {
                left: 16.0,
                right: 16.0,
                top: 8.0,
                bottom: 8.0,
            })
        );
    }

    // --- semantics ---------------------------------------------------------

    /// A deliberate *addition*, not a translation: v1's rows were a click handler
    /// and a pointer cursor, invisible to a screen reader.
    #[test]
    fn a_row_is_a_radio_in_single_mode_and_a_checkbox_in_multiple() {
        let role_of = |mode: SelectionMode| {
            let mut group = SelectGroup::new().row("One", || Box::new(Text::new("row")));
            group = match mode {
                SelectionMode::Single => group,
                SelectionMode::Multiple => group.multiple(true),
            };
            let pressable = group.children()[0].build().children().remove(0).build();
            pressable
                .as_any()
                .downcast_ref::<Pressable>()
                .expect("a pressable")
                .semantics_hint()
                .role
        };
        assert_eq!(role_of(SelectionMode::Single), SemanticsRole::Radio);
        assert_eq!(role_of(SelectionMode::Multiple), SemanticsRole::CheckBox);
    }

    /// The reader, not a captured value: a row is not rebuilt when the selection
    /// moves, so a value baked in at build time would report the opposite of what
    /// the user sees.
    #[test]
    fn a_row_reports_its_selected_state_by_reader() {
        let group = three_rows();
        let hint_of = |index: usize| {
            let mounted = group.children()[index].build().children().remove(0).build();
            mounted
                .as_any()
                .downcast_ref::<Pressable>()
                .expect("a pressable")
                .semantics_hint()
        };
        let hint = hint_of(1);
        assert!(
            !hint.state.contains(SemanticsState::SELECTED),
            "unselected says nothing: {hint:?}"
        );

        click_row(&group, 1);

        // Asked again *after* the click: the reader is read, not stored, so a
        // widget that was built before the selection moved reports the new state.
        let hint = hint_of(1);
        assert!(
            hint.state.contains(SemanticsState::SELECTED),
            "and the same widget, asked again after the click, says selected: {hint:?}"
        );
    }

    #[test]
    fn the_group_itself_is_a_labelled_nothing() {
        let hint = three_rows().semantics_hint();
        assert_eq!(hint.role, SemanticsRole::Group);
        assert!(
            hint.label.is_none(),
            "v1 had no group name, and an empty string is not a name: {hint:?}"
        );
    }
}

#[cfg(test)]
mod accordion_tests {
    //! The **stateful** reference widget.
    //!
    //! `tests/badge.rs` pins a description and `tests/controls.rs` a single
    //! interaction. This pins the part that is easy to get wrong and hard to see:
    //! a value that changes after mount, a subtree that mounts and unmounts with
    //! it, and a header that must *keep its identity* across the toggle.
    //!
    //! The tests stand in for the host, which is enough because the host's
    //! contribution here is a documented one — the update pass visits every node
    //! every frame and dirtiness is *reported by* `update()`. So what is checked
    //! is that each widget reports its own change, not that it was told.

    use floom_accessibility::SemanticsState;

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use floom_core::{Point, PointerEvent, Widget, WidgetAny};
    use floom_types::Rect;

    use super::*;
    use crate::primitives::Text;
    use crate::testkit::{AllocatingCreateContext, RecordingUpdateContext, TestEventContext};

    const ROW: Rect = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 300.0,
        y1: 48.0,
    };
    const AT: Point = Point { x: 20.0, y: 20.0 };

    /// One panel, with a counter that can tell "the body was *declared*" from
    /// "the body widget was *built*".
    fn one_panel() -> (Accordion, Arc<AtomicUsize>) {
        one_panel_with(false)
    }

    fn one_panel_with(multiple: bool) -> (Accordion, Arc<AtomicUsize>) {
        let builds = Arc::new(AtomicUsize::new(0));
        let sink = builds.clone();
        (
            Accordion::new()
                .multiple(multiple)
                .panel("Only", indicator(), move || {
                    sink.fetch_add(1, Ordering::SeqCst);
                    Box::new(Text::new("body"))
                }),
            builds,
        )
    }

    fn indicator() -> impl Fn(bool) -> Box<dyn WidgetAny> + Send + Sync {
        |open| Box::new(Text::new(if open { "v" } else { ">" }))
    }

    fn three_panels() -> Accordion {
        let mut accordion = Accordion::new();
        for title in ["One", "Two", "Three"] {
            accordion = accordion.panel(title, indicator(), || Box::new(Text::new("body")));
        }
        accordion
    }

    /// Mounts panel `index` the way the reconciler would.
    fn mount_panel(accordion: &Accordion, index: usize) -> AccordionPanelLeaf {
        let child = accordion.children()[index].clone();
        let mut widget = child.build();
        widget
            .as_any_mut()
            .downcast_mut::<AccordionPanelLeaf>()
            .expect("a panel slot mounts a panel leaf")
            .clone()
    }

    fn tick(leaf: &mut AccordionPanelLeaf) -> RecordingUpdateContext {
        let mut create = AllocatingCreateContext::new();
        let id = leaf.create(&mut create);
        let mut cx = RecordingUpdateContext::new();
        leaf.update(&mut cx, id);
        cx
    }

    /// Clicks panel `header` through the real dispatch path: a press and then a
    /// release, both consumed by the header's own `Pressable`.
    fn click_header(accordion: &Accordion, panel: usize) {
        let mut header = accordion.children()[panel]
            .build()
            .children()
            .remove(0)
            .build();
        let pressable = header
            .as_any_mut()
            .downcast_mut::<Pressable>()
            .expect("the header slot mounts a pressable");
        let mut create = AllocatingCreateContext::new();
        let id = pressable.create(&mut create);
        for phase in [PointerEvent::primary_down(AT), PointerEvent::primary_up(AT)] {
            let mut cx = TestEventContext::new(id, ROW, phase.into());
            pressable.event(&mut cx, id);
        }
    }

    // --- structure ---------------------------------------------------------

    #[test]
    fn every_declared_panel_gets_one_slot() {
        let accordion = three_panels();
        let children = accordion.children();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].key, ChildKey::index(0));
        assert_eq!(children[2].key, ChildKey::index(2));
        assert_eq!(accordion.titles(), vec!["One", "Two", "Three"]);
    }

    #[test]
    fn the_stack_declares_its_axis_and_the_token_gap() {
        let accordion = three_panels().style(AccordionStyle {
            gap: 7.0,
            ..AccordionStyle::default()
        });
        let req = accordion
            .layout_request()
            .expect("the accordion is a stack");
        assert_eq!(req.axis, Some(Axis::Vertical));
        assert_eq!(req.gap, Some(7.0));
    }

    #[test]
    fn a_collapsed_panel_mounts_only_its_header() {
        let (accordion, _) = one_panel();
        let mut leaf = mount_panel(&accordion, 0);
        assert_eq!(
            leaf.children().len(),
            1,
            "header only, before the panel has ever been opened"
        );
        let cx = tick(&mut leaf);
        assert!(
            !cx.reconciled(),
            "a panel standing still must not ask to be reconciled"
        );
    }

    #[test]
    fn opening_a_panel_reconciles_and_builds_its_content() {
        let (mut accordion, builds) = one_panel();
        accordion.set_open(0, true);
        let mut leaf = mount_panel(&accordion, 0);

        let cx = tick(&mut leaf);

        assert!(
            cx.reconciled(),
            "the body's mount state changed, so the subtree has to be re-declared"
        );
        assert_eq!(leaf.children().len(), 2, "header plus the revealed body");
        assert_eq!(
            builds.load(Ordering::SeqCst),
            0,
            "declaring a slot does not build it"
        );
        let body = leaf.children()[1].build();
        assert_eq!(builds.load(Ordering::SeqCst), 1, "mounting does");
        assert!(body.as_any().is::<Text>());
    }

    #[test]
    fn closing_a_panel_unmounts_its_content_again() {
        let (mut accordion, _) = one_panel();
        accordion.set_open(0, true);
        let mut leaf = mount_panel(&accordion, 0);
        tick(&mut leaf);
        assert_eq!(leaf.children().len(), 2);

        accordion.set_open(0, false);
        tick(&mut leaf);

        assert_eq!(
            leaf.children().len(),
            1,
            "the body is gone once the panel is closed"
        );
    }

    // --- the state machine -------------------------------------------------

    #[test]
    fn every_panel_starts_collapsed() {
        assert_eq!(three_panels().open_state(), vec![false, false, false]);
    }

    #[test]
    fn multiple_panels_may_be_open_together() {
        let mut accordion = three_panels().multiple(true);
        accordion.set_open(0, true);
        accordion.set_open(2, true);
        assert_eq!(
            accordion.open_state(),
            vec![true, false, true],
            "two open at once, the middle one shut"
        );
    }

    /// The imperative door applies the same rule a click does — it is the one
    /// place a caller could otherwise reach a state the UI cannot.
    #[test]
    fn the_imperative_door_applies_the_single_panel_rule_too() {
        let mut accordion = three_panels();
        accordion.set_open(0, true);
        accordion.set_open(1, true);
        assert_eq!(accordion.open_state(), vec![false, true, false]);
    }

    /// A programmatic change does **not** fire `on_change`: that callback reports
    /// what a person did, and a caller that just set the state knows already.
    #[test]
    fn the_imperative_door_does_not_fire_on_change() {
        let changes = Arc::new(AtomicUsize::new(0));
        let sink = changes.clone();
        let (mut accordion, _) = one_panel();
        accordion = accordion.on_change(move |_, _| {
            sink.fetch_add(1, Ordering::SeqCst);
        });

        accordion.set_open(0, true);
        assert_eq!(changes.load(Ordering::SeqCst), 0);
    }

    // --- clicking ----------------------------------------------------------

    #[test]
    fn clicking_a_header_opens_that_panel_and_reports_it() {
        let seen: Arc<Mutex<Vec<(usize, bool)>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        let (mut accordion, _) = one_panel();
        accordion = accordion.on_change(move |index, open| {
            sink.lock().unwrap().push((index, open));
        });

        click_header(&accordion, 0);

        assert!(accordion.is_open(0), "the click reached the shared state");
        assert_eq!(
            *seen.lock().unwrap(),
            vec![(0, true)],
            "and the callback reported which panel moved which way"
        );
    }

    #[test]
    fn clicking_the_open_panel_closes_it_again() {
        let (mut accordion, _) = one_panel();
        accordion.set_open(0, true);
        click_header(&accordion, 0);
        assert!(!accordion.is_open(0));
    }

    #[test]
    fn a_click_does_not_dirty_the_accordion_itself() {
        let mut accordion = one_panel().0;
        let mut create = AllocatingCreateContext::new();
        let id = accordion.create(&mut create);
        let mut cx = RecordingUpdateContext::new();
        accordion.update(&mut cx, id);
        assert_eq!(
            cx.dirty_count(),
            0,
            "the accordion owns the same panel slots either way; each panel \
             reports its own change, so a report here would be redundant work"
        );
    }

    // --- the header survives its own toggle --------------------------------

    /// The header must not be rebuilt when the panel opens: a rebuilt header is a
    /// new node, so the keyboard focus the press requested is gone — thrown away
    /// by the very act of activating it.
    #[test]
    fn the_header_slot_key_does_not_depend_on_the_open_flag() {
        let (mut accordion, _) = one_panel();
        let closed = mount_panel(&accordion, 0).children()[0].clone();

        accordion.set_open(0, true);
        let open = mount_panel(&accordion, 0).children()[0].clone();

        assert_eq!(
            closed.key, open.key,
            "the reconciler reuses a child only when the key still matches"
        );
        assert_eq!(closed.key, ChildKey::index(0));
    }

    /// A header that is not rebuilt cannot know a value it captured at build
    /// time, so its reported state has to be read from the shared flag. Without
    /// this the header would announce "collapsed" on a visibly open panel.
    #[test]
    fn a_header_built_before_the_toggle_still_reports_the_new_state() {
        let (mut accordion, _) = one_panel();
        let mut header = mount_panel(&accordion, 0).children()[0].clone().build();
        let pressable = header
            .as_any_mut()
            .downcast_mut::<Pressable>()
            .expect("the header is a pressable");

        assert!(
            pressable
                .semantics_hint()
                .state
                .contains(SemanticsState::COLLAPSED)
        );

        accordion.set_open(0, true);

        assert!(
            pressable
                .semantics_hint()
                .state
                .contains(SemanticsState::EXPANDED),
            "the same widget instance now reports what the user can see"
        );
    }

    /// The indicator is a widget that re-declares its own child, because a bare
    /// `Child` cannot change: the host only rebuilds a child whose key changed,
    /// and everything inside a stable header has a stable key.
    ///
    /// Reaching it is the argument in miniature — the header is a `Pressable`,
    /// its child is the row `Stack`, and the indicator is the row's third slot.
    #[test]
    fn the_indicator_rebuilds_its_glyph_when_the_panel_opens() {
        let (mut accordion, _) = one_panel();
        let leaf = mount_panel(&accordion, 0);
        let mut header = leaf.children().remove(0).build();
        let pressable = header
            .as_any_mut()
            .downcast_mut::<Pressable>()
            .expect("the header slot mounts a pressable");
        let mut row = pressable.children().remove(0).build();
        let row = row
            .as_any_mut()
            .downcast_mut::<Stack>()
            .expect("the pressable's child is the row")
            .clone();
        assert_eq!(row.children().len(), 3, "title, spacer, indicator");
        let mut indicator_child = row.children().remove(2).build();
        let slot = indicator_child
            .as_any_mut()
            .downcast_mut::<AccordionIndicatorSlot>()
            .expect("the third slot is the indicator");

        let collapsed = slot.children()[0].clone();
        let mut create = AllocatingCreateContext::new();
        let id = slot.create(&mut create);
        let mut cx = RecordingUpdateContext::new();
        slot.update(&mut cx, id);
        assert!(!cx.reconciled(), "closed is where it started");

        accordion.set_open(0, true);
        let mut cx = RecordingUpdateContext::new();
        slot.update(&mut cx, id);

        assert!(
            cx.reconciled(),
            "a different glyph is a different child, so the slot has to \
             re-declare it"
        );
        let opened = slot.children()[0].clone();
        assert_ne!(
            collapsed.key, opened.key,
            "and the key carries the flag, so the host mounts rather than reuses"
        );
    }

    // --- what cannot be animated -------------------------------------------

    /// Without a transition the reveal is instant, matching v1's `dyn_view`.
    #[test]
    fn without_a_transition_nothing_is_declared_while_shown() {
        let (mut accordion, _) = one_panel();
        accordion.set_open(0, true);
        let mut leaf = mount_panel(&accordion, 0);
        tick(&mut leaf);
        assert!(leaf.animation().is_none(), "a mount is not a motion");
    }

    /// With one, the *panel's own node* declares it — which is why the leaf holds
    /// the reveal as a value instead of mounting it as a node of its own.
    #[test]
    fn a_content_transition_is_declared_on_the_panel_node() {
        let (mut accordion, _) = one_panel();
        accordion = accordion.content_transition(PresenceTransition::fade(0.2));
        accordion.set_open(0, true);
        let mut leaf = mount_panel(&accordion, 0);
        tick(&mut leaf);

        let spec = leaf.animation().expect("the reveal declares its motion");
        assert!(
            spec.duration_secs > 0.0,
            "and it is the caller's duration, not a default"
        );
    }

    /// The control for the whole point: **2.0 cannot animate the reveal's
    /// height.** v1's `md-expand_transition` tweened `height: 0 -> computed`,
    /// which needs a re-layout every frame, and the host only lays out when
    /// layout is dirty (`WidgetHost::frame` -> `if self.layout_dirty`). So the
    /// panel's box is its final height from the first frame and the transition
    /// can only be compositor-only.
    #[test]
    fn the_reveal_does_not_resize_the_panel_over_time() {
        let (mut accordion, _) = one_panel();
        accordion = accordion.content_transition(PresenceTransition::fade(0.2));
        accordion.set_open(0, true);
        let mut leaf = mount_panel(&accordion, 0);

        // Mid-flight: the transition is declared and the body is mounted...
        tick(&mut leaf);
        assert!(leaf.animation().is_some());
        assert_eq!(leaf.children().len(), 2);

        // ...while the panel's own layout request is unchanged, because nothing
        // here scales a box by the animation. A height reveal would have to.
        let req = leaf.layout_request().expect("a vertical stack");
        assert_eq!(req.axis, Some(Axis::Vertical));
        assert_eq!(
            req.height, None,
            "the panel claims no height of its own, so the host has nothing to \
             re-derive per frame — which is exactly why this is unavailable"
        );
    }
}
