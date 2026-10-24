//! Headless app shell for Floom 2.0.
//!
//! This is the missing piece that makes the 2.0 crates *runnable*: it owns a
//! widget tree, drives layout, records paint output into a retained
//! [`Scene`], routes input through the runtime's capture/bubble router, manages
//! focus, and publishes a semantics tree. Nothing here touches a window system
//! or a GPU — a real backend supplies a surface and pumps [`WidgetHost::frame`].
//!
//! ```text
//! mount(widget)  →  ElementId from runtime-core IdAllocator
//! frame()        →  update() → layout() → paint into Scene
//! dispatch(ev)   →  hit-test → route → widget.event()
//! ```

#![forbid(unsafe_code)]

pub mod image_codec;
pub mod runner;

pub use image_codec::{DecodedImage, decode_image};
pub use runner::{AppRunner, RunStats};

use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use floom_accessibility::{SemanticsNode, SemanticsTree};
use floom_animation::{AnimId, AnimKind, AnimSample, AnimationScheduler};
// The host's own tree, as plain data, lives in `floom-core`: the host hands it
// over and the L8 tooling describes it, and neither side should have to know the
// other. Re-exported here so `floom_app::HostedNode` keeps working.
use floom_core::{
    Align, AnimationSpec, Axis, Child, ChildKey, CreateContext, ElementId, Event, EventContext,
    FontWeight, Key, KeyEvent, KeyPhase, LayoutRequest, Length, Modifiers, NamedKey, NativeGrant,
    NativeRequest, NativeSnapshot, OverlayHandle, OverlayKind, OverlayRequest, PaintContext, Point,
    PointerPhase, Position, Rect, ResourceId, ScrollOffset, SemanticsHint, SemanticsRole, Size,
    UpdateContext,
};
pub use floom_core::{HostedKind, HostedNode};
use floom_focus::{FocusDirection, FocusManager, FocusScopeId};
use floom_layout::{
    AlignItems as LAlign, FlexDirection, JustifyContent, LayoutEngine, LayoutNode, LayoutStyle,
    MeasureFn, Position as LPosition,
};
use floom_native::{
    InputMode, NativeCompositionMode, NativeError, NativeSlot, NativeState, NativeViewController,
    NativeViewId, NativeViewKind, NativeViewPool, NativeViewProvider,
};
use floom_overlay::{CompositorProp, OverlayManager};
use floom_runtime_core::IdAllocator;
use floom_runtime_core::{EventRouter, Hit, HitRegion, RouteStop};
use floom_scene::{Resource, ResourceKind, ResourceRegistry, Scene, SceneNode, SceneNodeId};
use floom_text::{LayoutCache, TextAlign, TextLayoutOptions, TextShaper};
use floom_types::NativeCapture;
use floom_types::ScrollEvent;
use floom_types::{ClipRegion, CursorStyle, StyledText, resolve_clip};
use floom_widgets::WidgetAny;
use smallvec::SmallVec;

/// Stable handle to a mounted widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameStats {
    pub updated: usize,
    pub painted: usize,
    pub layout_nodes: usize,
    /// Widgets that asked for the layout pass to run again this frame, via
    /// [`floom_core::UpdateContext::mark_layout_dirty`].
    ///
    /// Separate from `relaid` because a request does not guarantee a pass: the
    /// frame may already have been going to lay out. A test that wants "this
    /// change reached layout" asserts on the geometry; one that wants "this
    /// widget said its layout moved" asserts on this.
    pub layout_requested: usize,
    /// Whether this frame ran the layout pass.
    ///
    /// Separate from `layout_nodes`, which is the size of the layout tree
    /// whether or not anything was laid out this frame: a test that asserts
    /// "layout ran" against a node count is asserting something that cannot
    /// fail, and this is what it should assert instead.
    pub relaid: bool,
    /// Participants whose state changed this frame.
    ///
    /// Not scene damage: in Overlay mode a participant is a sibling above the
    /// framework's surface, so moving it changes no pixels in the scene the
    /// renderer drew. The compositor still owes a present, and a runner keying
    /// its skip logic off `scene.damage()` alone would suppress it.
    pub native_applied: usize,
    /// Participants that had to change composition mode because the provider
    /// cannot crop and the participant landed under a clipping ancestor.
    ///
    /// Counted rather than logged: one of these per participant is normal for a
    /// plane-based backend, and a *rising* number means participants are being
    /// re-created repeatedly — the churn this is designed to avoid.
    pub native_degraded: usize,
    /// Participants hidden because they needed cropping, the provider cannot
    /// crop, and no mode the framework composites was on offer.
    ///
    /// Non-zero is a visible gap on screen, and it is the honest answer: the
    /// alternatives are drawing outside the participant's box (device case D6)
    /// or rescaling its content, and both are silent.
    pub native_clip_hidden: usize,
}

struct Node {
    id: ElementId,
    widget: Box<dyn WidgetAny>,
    layout_node: LayoutNode,
    bounds: Rect,
    parent: Option<usize>,
    children: Vec<usize>,
    /// Key per mounted child slot, parallel to [`Node::children`]. Lets the
    /// reconciler keep a widget mounted across `children()` re-calls by key
    /// instead of by position.
    child_keys: Vec<ChildKey>,
    style: LayoutStyle,
    /// The parent-declared layout intent for this node, if it was mounted as a
    /// composite child. `None` for the root and imperatively-mounted nodes,
    /// whose style is supplied directly. Used to refresh this node's style when
    /// the widget's own [`Widget::layout_request`] changes.
    parent_request: Option<LayoutRequest>,
    /// Tree depth, used to give deeper nodes priority in hit-testing.
    depth: u32,
    /// How far this node scrolls its own subtree, set by a scrolling widget.
    ///
    /// Applied during the absolute-position accumulation pass, not to the
    /// layout: a scroll offset is not a box, and folding it into a margin would
    /// make the viewport grow by the scroll distance. It applies to this node's
    /// *descendants*, never to the node itself — a scroll container's own frame
    /// does not move when its content scrolls, which is what makes it read as a
    /// frame around a window rather than as part of the content.
    scroll_offset: ScrollOffset,
    /// Set by the last `update()`; drives whether we repaint this frame.
    dirty: bool,
    /// Set when the widget's `children()` set changed; the reconciler rebuilds
    /// this node's subtree on the next frame.
    children_dirty: bool,
    /// Unmounted but not yet removed from the vector; skipped by every pass so
    /// its `NodeId` stays valid for live nodes.
    dead: bool,
    /// True for the root of an overlay subtree: painted last (above the main
    /// tree), laid out independently against the viewport, and given hit/focus
    /// priority so its content escapes the parent's clip and can actually be
    /// clicked. Its descendants inherit this by ancestry, not by this flag.
    is_overlay: bool,
    /// Scene nodes this widget painted last frame. Painting reuses them
    /// positionally via `Scene::upsert` (so unchanged payloads produce no
    /// damage), and leftovers are retired.
    scene_nodes: Vec<SceneNodeId>,
    /// The `ClipPush`/`ClipPop` pair bracketing this node's subtree in the
    /// scene's draw order, when the node's widget declares
    /// [`Widget::clip`]. Both are `None` for every unclipped node.
    ///
    /// They live here, not in `scene_nodes`, because a widget's own `paint`
    /// cannot bracket the *descendants* it does not paint: the host walks the
    /// subtree, so the host is the only party that knows where the bracket
    /// ends. Holding the ids (rather than minting them per frame) is what
    /// keeps the pair in place across frames — a scene node's position in the
    /// draw order is the slot it was allocated in, so re-inserting them every
    /// frame would append the pop *after* the whole scene instead of after
    /// this subtree.
    clip_push: Option<SceneNodeId>,
    clip_pop: Option<SceneNodeId>,
    /// The `GroupPush`/`GroupPop` pair bracketing this node's scene nodes
    /// while a transform-class composite (scale, rotate) animates it, when the
    /// widget paints. Same story as `clip_push`/`clip_pop`: the host writes
    /// the bracket because only the walk knows where the node's picture begins
    /// and ends, and the ids are held (not re-minted) so the markers keep
    /// their draw-order slots across frames.
    group_push: Option<SceneNodeId>,
    group_pop: Option<SceneNodeId>,
    /// The `FilterPush`/`FilterPop` pair bracketing this node's subtree while
    /// its widget declares [`Widget::filter`]. Same story as the two pairs
    /// above: only the walk knows where the subtree's picture begins and ends,
    /// and the ids are held so the markers keep their draw-order slots.
    filter_push: Option<SceneNodeId>,
    filter_pop: Option<SceneNodeId>,
    /// Set when a native participant is bound to this node. Its hit region
    /// then resolves to the native view — which takes the pointer itself —
    /// instead of to this widget. Cleared when the participant is retired.
    native: Option<NativeViewId>,
    /// Set while this node's participant is bound but off screen — clipped
    /// away by an ancestor, or hidden under a modal.
    ///
    /// Such a node must not be reachable by the pointer at all. Withdrawing its
    /// *native* claim is not enough: the node is still a widget in the tree, and
    /// a deep node's region outranks a shallow one's, so an invisible surface
    /// would keep eating clicks meant for whatever is painted where it used to
    /// be — with nothing on screen to explain it. `false` for every node that
    /// is not a hidden participant.
    native_hidden: bool,
    /// Set for `Text` leaves whose width/height is not pinned: they size
    /// themselves through a taffy measure callback reading this shared state,
    /// so extent changes (content edit, container reflow) re-measure instead
    /// of freezing the mount-time measurement.
    text_measure: Option<Arc<Mutex<TextMeasure>>>,
    /// What this node's own animation is doing to it this frame, if it has one.
    ///
    /// Parked on the node rather than looked up from the running list so the
    /// paint pass reads it in constant time: looking it up per node would put
    /// a cost on every node in the tree for a feature most of them never use.
    motion: Option<MotionSample>,
}

/// This frame's sample of the animation running on one node.
#[derive(Debug, Clone)]
struct MotionSample {
    /// The sampled value — what the element's own `paint` reads through
    /// `PaintContext::animation_value`.
    value: f64,
    /// The compositor channels the animation is driving this frame, sampled to
    /// their per-channel values (B6). Empty means the motion is the element's
    /// to draw rather than the host's to fold in.
    props: SmallVec<[CompositorProp; 2]>,
}

/// One node's animation, as the host is running it.
#[derive(Debug)]
struct NodeAnimation {
    node: usize,
    /// The scheduler's id, while it is running.
    ///
    /// `None` once a *declared* animation has finished. The declaration is
    /// still standing, so the entry stays: dropping it would let the next
    /// reconcile meet a declaration it does not recognise and restart the
    /// animation — which for a finite repeat would mean it played again every
    /// time its widget was updated.
    id: Option<AnimId>,
    /// What the widget declared, for entries that came from a declaration.
    /// `None` for the ones started through [`WidgetHost::animate`], which the
    /// host retires as soon as they are over.
    declared: Option<AnimationSpec>,
}

/// How far [`WidgetHost::frame`] advances the animation clock.
///
/// A nominal 60 Hz step, not measured elapsed time: an entry animation is
/// measured in frames, and a wall-clock delta would make it fast on an idle
/// loop and slow under load — and untestable either way.
pub const NOMINAL_FRAME_SECS: f64 = 1.0 / 60.0;

/// Live measure inputs for a text leaf, shared between the widget (updated on
/// content change) and the layout engine's measure closure.
#[derive(Debug)]
struct TextMeasure {
    text: String,
    font_size: f32,
    /// The width the measure closure last wrapped against — the `avail.width` it
    /// was handed, *before* the layout engine rounds the leaf's box to whole
    /// pixels.
    ///
    /// The paint path needs this. It re-shapes the same string to get glyph runs,
    /// and it used to wrap against the node's own (rounded) box — so a string
    /// whose natural width landed in the upper half-pixel of that box wrapped by
    /// one word: `498 of 2997` random regular-weight strings, measured.
    ///
    /// It is the *available* width rather than the measured result, and that is
    /// the whole subtlety. Recording what the measure returned looked equivalent
    /// and is not: for a string that is wider than its box the result is the
    /// wrapped width (fine), but for one measured against a wide available space
    /// the result is the string's natural width, and feeding *that* back as the
    /// paint wrap width stops the label ever wrapping at all. The available width
    /// is what the text was actually wrapped against, so the two agree exactly.
    wrap_width: f64,
    /// Part of the state because it changes the measured width: a `Bold` run is
    /// wider than a `Normal` one, so a weight that changes after mount has to
    /// re-measure the node or the box stays sized for the old glyphs while the
    /// paint path draws the new weight's — text clipped to a width it no longer
    /// needs, which is exactly the bug a *missing* comparison here produces.
    weight: FontWeight,
}

/// A mounted overlay: an independent widget subtree painted above the main
/// tree, laid out against the viewport at `bounds`, with hit/focus priority.
struct OverlaySlot {
    id: OverlayHandle,
    /// Index into `nodes` of the overlay's root widget.
    node: usize,
    /// Viewport-relative rect the content is positioned and hit-tested against.
    bounds: Rect,
    /// Focus before the overlay opened, restored when it closes.
    previous_focus: Option<ElementId>,
    /// The widget that asked for this overlay, if any. Its content is handed
    /// back to this node through [`floom_core::Widget::overlay_closed`] when
    /// the overlay comes down, which is how an opener learns the result of a
    /// popup it does not own the events for.
    owner: Option<ElementId>,
}

/// What a participant's slot is pooled under.
///
/// The two spaces are separate on purpose. A caller that numbers its rows from
/// zero would otherwise collide with the host's own counter, and two
/// participants that are on screen at the same time would be handed the same
/// slot — the second `bind` then fails, and the second participant silently
/// never appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SlotKey {
    /// The widget brought an identity: a virtualized row's key.
    Caller(u64),
    /// The widget had none, so the host minted one.
    Minted(u64),
}

/// A live native participant: a node the host has handed to a provider.
///
/// The host owns this, not the widget, because a native view has to outlive the
/// frame that created it, be re-bound when its slot is recycled for another
/// row, and be destroyed exactly once. A widget holding one directly would leak
/// it on every unmount — silently, because a leaked native view looks exactly
/// like a working one until the device runs out of them.
#[derive(Debug, Clone, Copy)]
struct Participant {
    /// Index into `nodes` of the node this participant occupies.
    node: usize,
    slot: NativeSlot,
    /// The key the widget asked for. `0` means "I have no identity", and the
    /// host mints one so two identity-less surfaces never share a slot.
    requested_key: u64,
    /// What the slot is actually pooled under.
    key: SlotKey,
    kind: NativeViewKind,
    /// The modes the widget will accept, strongest first. Kept because it is
    /// needed again after negotiation: a participant whose provider cannot crop
    /// has to be re-negotiated into a mode the *framework* composites, and only
    /// the caller knows which of those it will take.
    modes: &'static [NativeCompositionMode],
    /// The mode the provider negotiated at bind time. Recorded rather than
    /// assumed: on Wayland the answer is not the one that was asked for.
    mode: NativeCompositionMode,
    input: InputMode,
    /// Last state pushed down. `None` until the first apply.
    last: Option<NativeState>,
    /// Set when the participant needs cropping, its provider cannot crop, and
    /// it will accept no mode the framework composites — so it is hidden.
    ///
    /// Remembered rather than re-derived so the decision costs one frame: a
    /// per-frame re-negotiation of a participant that is hopeless cannot
    /// succeed, and would hide it again every frame forever.
    clip_starved: bool,
}

/// A participant whose live view has been withdrawn in favour of a texture.
///
/// The OS cannot animate a view it owns: no windowing API will move an
/// `NSView` along a curve, fade it out mid-transition, or rotate it, and the
/// closest available trick — re-placing it every frame — still leaves it unable
/// to do any of the things a page transition asks for. So a transition freezes
/// the participant into a texture and the framework paints the texture instead.
///
/// The *host* owns this record rather than the widget, and for the same reason
/// it owns the participant: the live view is still bound underneath. A widget
/// that owned the frozen texture would strand it on unmount, and the provider
/// would be left holding a view nobody can name.
struct Frozen {
    node: usize,
    slot: NativeSlot,
    /// The texture standing in for the live view.
    snapshot: NativeSnapshot,
    /// The transition running against that texture, if any. `None` for a
    /// freeze with no animation: the texture is simply drawn where the view
    /// was until somebody thaws it.
    anim: Option<AnimId>,
    /// This frame's sampled tween — the same channel an overlay's entry
    /// animation uses, and for the same reason: whatever ticks the clock and
    /// whatever paints must read one value, not derive two.
    composite: Option<CompositorProp>,
}

/// What a widget asked the host to do with an overlay during an event.
///
/// Queued rather than applied inline: opening an overlay mounts a subtree and
/// mutates the host, and the widget asking for it is borrowed out of that same
/// host for the duration of the event. The queue lets the request outlive the
/// borrow and is drained once the event has finished dispatching.
enum OverlayCommand {
    Open {
        /// Minted during dispatch, and already in the content's hands.
        handle: OverlayHandle,
        /// Who asked — receives `overlay_closed` when this overlay comes down.
        owner: ElementId,
        kind: OverlayKind,
        modal: bool,
        bounds: Rect,
        /// Already built, with its handle: the content factory ran while the
        /// event was still dispatching.
        content: Box<dyn WidgetAny>,
    },
    Close(OverlayHandle),
}

/// The tombstone left behind in a node whose widget was handed back to an
/// overlay's opener.
///
/// Unmounting leaves dead nodes in the vector so live `NodeId`s stay valid, and
/// a `Node` must keep *some* widget; this stands in for one that has been given
/// away. Nothing ever reaches it — `dead` short-circuits every pass that walks
/// the tree.
struct Retired;

impl floom_core::Widget for Retired {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}
}

impl WidgetAny for Retired {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<SemanticsHint> {
        None
    }
}

/// Owns and drives a widget tree.
pub struct WidgetHost {
    nodes: Vec<Node>,
    root: Option<usize>,
    ids: IdAllocator,
    router: EventRouter,
    layout: LayoutEngine,
    scene: Scene,
    resources: ResourceRegistry,
    /// Interned glyph runs, keyed by a hash of (text, font size): a per-frame
    /// lookup must not allocate a key String per text node just to find out it
    /// is already cached. The entry carries the text for verification, so a
    /// u64 collision degrades to a re-intern, never to a wrong resource.
    glyph_cache: HashMap<u64, GlyphCacheEntry>,
    /// Interned text nodes for *styled* descriptions, keyed by a hash of the
    /// description. A styled description is more than a string — it carries
    /// the ranges that differ from the base style — so it cannot share the
    /// plain text's cache, and it needs its own because the shape it produces
    /// is one text node *per ink*.
    styled_glyph_cache: HashMap<u64, StyledGlyphCacheEntry>,
    /// Parley-backed shaper that produces the runs stored in `resources`.
    shaper: TextShaper,
    /// Shaped-text cache keyed by (text + layout options). `fill_text` reuses
    /// shaped runs across frames instead of re-shaping on every repaint, which
    /// also keeps `GlyphRun` font data stable so the glyph raster cache keeps
    /// hitting.
    text_layout_cache: LayoutCache,
    /// Scene nodes orphaned by unmounts that happened outside a paint pass;
    /// retired at the start of the next paint (inside its transaction).
    pending_scene_removals: Vec<SceneNodeId>,
    semantics: SemanticsTree,
    /// Whether the semantics tree needs a *full* rebuild before it is read
    /// again — the tree's shape changed (mount, unmount, reconcile, overlay
    /// open/close). Content-only changes go through `semantics_stale` instead,
    /// which upserts individual entries without touching the rest.
    semantics_dirty: bool,
    /// Mounted node indices whose widget reported dirty since the last frame
    /// and whose semantics entry therefore needs an incremental refresh.
    semantics_stale: Vec<usize>,
    /// Scratch buffers reused across `run_layout` passes: taffy reports
    /// parent-relative rects, and accumulating them into absolute ones needs
    /// one slot per node. Resized (not reallocated) per pass.
    layout_relative: Vec<Option<Rect>>,
    layout_absolute: Vec<Option<Rect>>,
    /// Absolute position after scroll offsets, which is what is painted and
    /// hit-tested. Separate from `layout_absolute` so a child still resolves
    /// against its parent's *unscrolled* box — a scroll container's content is
    /// laid out in the container's own coordinates and only then moved.
    painted_absolute: Vec<Option<Rect>>,
    /// Reused traversal stack for [`Self::content_size`].
    ///
    /// Host-owned and cleared, never reallocated, so a subtree walk costs no
    /// allocation. This matters because the walk is driven by a widget asking a
    /// question mid-update, and a fresh `Vec` per question would put an
    /// allocation on the settled-frame path that `tests/allocation.rs` budgets at
    /// eight for the whole tree.
    content_stack: Vec<usize>,
    /// Per node, the summed scroll offset of its ancestors. Read by a node's
    /// children and so one slot behind `scroll_offset` in the same way a node's
    /// absolute position trails its parent's.
    scroll_inherited: Vec<ScrollOffset>,
    /// True while the embedder says the app is not visible or the OS has
    /// suspended it: animation clocks stop advancing (see
    /// [`WidgetHost::set_paused`]), and the runner drops frame requests.
    paused: bool,
    focus: FocusManager,
    scope: FocusScopeId,
    /// True when focus was last moved by keyboard navigation (Tab), so a focus
    /// ring can be shown for keyboard users without lighting up on every mouse
    /// click. Set by [`WidgetHost::focus_next`] / [`WidgetHost::focus_prev`] and
    /// cleared by any pointer-driven focus in [`WidgetHost::set_focus`].
    keyboard_navigation: bool,
    /// Stacked overlays (popups, dialogs, sheets) painted above the main tree.
    overlay: OverlayManager,
    /// Mounted overlay subtrees, parallel to `overlay`'s entry order.
    overlays: Vec<OverlaySlot>,
    /// Overlay requests widgets made during the event being dispatched; drained
    /// by [`WidgetHost::dispatch`] once every widget has had its turn.
    overlay_commands: Vec<OverlayCommand>,
    /// Next overlay handle to mint. Handles are issued by the host — not by the
    /// manager — because the opener and the content both need theirs *before*
    /// the overlay is opened, and only the host sees both.
    next_overlay_handle: u64,
    /// Composition caret rectangle the focused text widget last reported via
    /// [`EventContext::set_ime_cursor_area`]; `None` when nothing is composing.
    /// The runner reads it after an IME event to anchor the OS popup.
    ime_caret: Option<Rect>,
    /// The host's animation clock, in seconds since the first frame.
    ///
    /// Advanced a nominal frame at a time by [`WidgetHost::frame`] rather than
    /// read off the wall clock: an entry animation is a *frame* animation, and
    /// driving it from elapsed time would make it depend on how fast the loop
    /// happens to be running and leave it untestable. An embedder that knows
    /// its real frame delta can advance it itself through
    /// [`WidgetHost::tick_animations`].
    anim_clock: f64,
    /// Whether any overlay is still playing its entry animation, i.e. whether
    /// the frame loop owes another frame for no other reason.
    overlay_animating: bool,
    /// The clock the native-transition tweens run on: the same scheduler the
    /// overlay stack uses, ticked from the same frame step. Reusing it is the
    /// point — a second animation mechanism is the likeliest way to end up
    /// with a participant that never unfreezes.
    native_anim: AnimationScheduler,
    /// Animations running against whole nodes: what [`WidgetHost::animate`]
    /// starts, and what a widget's own declaration starts.
    ///
    /// Its own scheduler rather than a shared one, because the overlay manager
    /// owns its table and the native transitions need theirs. What the three
    /// have in common is the clock, not the state.
    motion: AnimationScheduler,
    /// One entry per animating node — empty on a tree where nothing moves,
    /// which is what keeps this channel free for everybody who does not use it.
    node_anims: Vec<NodeAnimation>,
    /// Scratch for the sampler's output, so an animation frame does not
    /// allocate (see [`AnimationScheduler::tick_into`]).
    motion_samples: Vec<AnimSample>,
    /// Scratch for the declared animations that finished this tick, so the
    /// completion callback can run after the sampler has let go of every
    /// borrow — calling a widget while `motion_samples` is still borrowed would
    /// re-enter the host it already holds. Empty on a tree where nothing ended.
    finished_declared: Vec<usize>,
    /// Native composition. `None` means every participant request is refused,
    /// which is the honest answer for a host with no OS bridge rather than a
    /// participant that silently draws nothing.
    native: Option<NativeViewController<Box<dyn NativeViewProvider>>>,
    /// Live participants, in the order they were bound.
    participants: Vec<Participant>,
    /// Participants frozen into a texture for the duration of a transition.
    frozen: Vec<Frozen>,
    /// Slots keyed by recycling key, so a row scrolling out of view hands its
    /// slot to the row scrolling in instead of minting another.
    native_pool: NativeViewPool<SlotKey>,
    /// Identity minted for participants that did not bring one.
    next_native_key: u64,
    /// Device-pixel scale for participant rects. The host passes it down
    /// because the logical→device boundary must be crossed exactly once, and a
    /// provider cannot know it.
    scale: f64,
    /// The participant holding OS focus, if any. Exclusive: while one holds it
    /// the framework suspends its own IME ownership, because two owners pushing
    /// candidate-window geometry makes the candidate window jump.
    native_focus: Option<NativeSlot>,
    /// The framework element that had focus when a participant took it, so
    /// focus can be handed back when the participant blurs. Without this a
    /// focus round-trip through a native view drops the framework's focus
    /// entirely and the next keystroke goes nowhere.
    native_focus_return: Option<ElementId>,
    /// Nodes whose `native` flag must be cleared and whose hit region must be
    /// refreshed. Retiring a participant can happen while its node is borrowed
    /// (a widget re-stating a changed request mid-`update`), so the tree write
    /// is deferred to the next sync instead of done inline.
    pending_native_clears: Vec<usize>,
    size: Size,
    layout_dirty: bool,
    /// A widget was borrowed mutably since the last frame, i.e. something
    /// outside the frame loop touched the tree (an app callback, a service, a
    /// timer). `widget_mut` is the only door into a mounted widget, so this is
    /// the framework's one signal that content may have changed without a
    /// platform event to announce it — it is what lets the runner ask for the
    /// next frame instead of waiting for input.
    content_dirty: bool,
    last_stats: FrameStats,
}

impl Default for WidgetHost {
    fn default() -> Self {
        Self::new()
    }
}

// --- layout intent lowering ------------------------------------------------

fn to_a11y_role(role: SemanticsRole) -> floom_accessibility::SemanticsRole {
    use SemanticsRole as C;
    use floom_accessibility::SemanticsRole as A;
    match role {
        C::None => A::None,
        C::Button => A::Button,
        C::CheckBox => A::CheckBox,
        C::Radio => A::Radio,
        C::Switch => A::Switch,
        C::Slider => A::Slider,
        C::TextField => A::TextField,
        C::TextView => A::TextView,
        C::Image => A::Image,
        C::ImageButton => A::ImageButton,
        C::Link => A::Link,
        C::Heading => A::Heading,
        C::StaticText => A::StaticText,
        C::List => A::List,
        C::ListItem => A::ListItem,
        C::Group => A::Group,
        C::Scrollable => A::Scrollable,
        C::ProgressBar => A::ProgressBar,
        C::Tab => A::Tab,
        C::TabBar => A::TabBar,
        C::Dialog => A::Dialog,
        C::MenuItem => A::MenuItem,
        C::Custom(v) => A::Custom(v),
    }
}

fn to_a11y_state(state: floom_core::SemanticsState) -> floom_accessibility::SemanticsState {
    use floom_accessibility::SemanticsState as A;
    use floom_core::SemanticsState as C;
    let mut out = A::empty();
    let mut add = |flag: C, a11y: A| {
        if state.contains(flag) {
            out |= a11y;
        }
    };
    add(C::SELECTED, A::SELECTED);
    add(C::CHECKED, A::CHECKED);
    add(C::DISABLED, A::DISABLED);
    add(C::HIDDEN, A::HIDDEN);
    add(C::FOCUSED, A::FOCUSED);
    add(C::EXPANDED, A::EXPANDED);
    add(C::COLLAPSED, A::COLLAPSED);
    add(C::ENABLED, A::ENABLED);
    add(C::FOCUSABLE, A::FOCUSABLE);
    add(C::SCROLLABLE, A::SCROLLABLE);
    add(C::LIVE, A::LIVE);
    out
}

fn align_to_justify(align: Align) -> JustifyContent {
    match align {
        Align::Stretch => JustifyContent::Start,
        Align::Start => JustifyContent::Start,
        Align::Center => JustifyContent::Center,
        Align::End => JustifyContent::End,
        // The layout layer has carried `SpaceBetween` from the start; only the
        // engine-neutral vocabulary was missing it.
        Align::SpaceBetween => JustifyContent::SpaceBetween,
    }
}

fn align_to_align_items(align: Align) -> LAlign {
    match align {
        Align::Stretch => LAlign::Stretch,
        Align::Start => LAlign::Start,
        Align::Center => LAlign::Center,
        Align::End => LAlign::End,
        // Cross-axis "distribute between" does not exist in flexbox, and a caller
        // who asked for it on the cross axis gets the start edge rather than a
        // silently invented behaviour.
        Align::SpaceBetween => LAlign::Start,
    }
}

/// Lowers a composite child's [`LayoutRequest`] into the engine's [`LayoutStyle`].
///
/// This is the heart of engine neutrality: `floom-core` never names a concrete
/// layout type, so the lowering happens here, where both vocabularies are in
/// scope.
fn request_to_style(req: &LayoutRequest) -> LayoutStyle {
    let mut style = LayoutStyle {
        width: req.width,
        height: req.height,
        min_width: req.min_width,
        min_height: req.min_height,
        max_width: req.max_width,
        max_height: req.max_height,
        flex_grow: req.flex_grow.unwrap_or(0.0),
        aspect_ratio: req.aspect_ratio,
        ..LayoutStyle::default()
    };
    if let Some(axis) = req.axis {
        style.flex_direction = match axis {
            Axis::Horizontal => FlexDirection::Row,
            Axis::Vertical => FlexDirection::Column,
        };
    }
    if let Some(gap) = req.gap {
        style.gap = gap;
    }
    if let Some(margin) = req.margin {
        style.margin = margin;
    }
    if let Some(padding) = req.padding {
        style.padding = padding;
    }
    if let Some(align) = req.main_align {
        style.justify_content = align_to_justify(align);
    }
    if let Some(align) = req.cross_align {
        style.align_items = align_to_align_items(align);
    }
    if let Some(position) = req.position {
        style.position = match position {
            Position::Relative => LPosition::Relative,
            Position::Absolute => LPosition::Absolute,
        };
    }
    if let Some(inset) = req.inset {
        style.inset = floom_layout::AbsoluteInset {
            left: inset.left,
            top: inset.top,
            right: inset.right,
            bottom: inset.bottom,
        };
    }
    style
}

/// Applies a widget's own container intent ([`Widget::layout_request`]) on top of
/// a base style.
///
/// Sizing is *fill-in, never override*: a request may supply an intrinsic
/// width/height (e.g. a `Text` reporting its measured extent), but only when
/// the base style leaves that dimension unspecified — explicit sizing in the
/// mount style always wins. Container intent (axis, gap, alignment, position)
/// applies on top as before.
fn apply_self_layout(mut base: LayoutStyle, req: Option<&LayoutRequest>) -> LayoutStyle {
    if let Some(req) = req {
        if base.width.is_none() {
            base.width = req.width;
            base.min_width = req.min_width;
            base.max_width = req.max_width;
        }
        if base.height.is_none() {
            base.height = req.height;
            base.min_height = req.min_height;
            base.max_height = req.max_height;
        }
        if let Some(axis) = req.axis {
            base.flex_direction = match axis {
                Axis::Horizontal => FlexDirection::Row,
                Axis::Vertical => FlexDirection::Column,
            };
        }
        if let Some(gap) = req.gap {
            base.gap = gap;
        }
        // A margin declared by the slot is the *only* word for "inset this
        // child", so unlike sizing it is applied outright: a base style that
        // carried its own margin is a caller restating the slot's intent.
        if let Some(margin) = req.margin {
            base.margin = margin;
        }
        if let Some(padding) = req.padding {
            base.padding = padding;
        }
        if let Some(position) = req.position {
            base.position = match position {
                Position::Relative => LPosition::Relative,
                Position::Absolute => LPosition::Absolute,
            };
        }
        if let Some(inset) = req.inset {
            base.inset = floom_layout::AbsoluteInset {
                left: inset.left,
                top: inset.top,
                right: inset.right,
                bottom: inset.bottom,
            };
        }
        if let Some(align) = req.main_align {
            base.justify_content = align_to_justify(align);
        }
        if let Some(align) = req.cross_align {
            base.align_items = align_to_align_items(align);
        }
    }
    base
}

/// [`apply_self_layout`] plus text-measure resolution.
///
/// A bare `Text` whose width/height is not pinned by the mount style or a
/// parent slot sizes itself through a taffy measure callback instead of a
/// frozen extent: the dims the widget's own `layout_request` would fill are
/// cleared (left `auto`) and a shared [`TextMeasure`] handle is returned for
/// the leaf's measure closure. Pinned dims stay explicit and win as before.
fn self_layout_with_text_measure(
    base: LayoutStyle,
    widget: &dyn WidgetAny,
) -> (LayoutStyle, Option<Arc<Mutex<TextMeasure>>>) {
    let req = widget.layout_request();
    // Dims that are explicit before the widget's own request gets a say stay
    // explicit — the measure fn must not fight a declared size.
    let pinned_width = base.width.is_some();
    let pinned_height = base.height.is_some();
    let mut style = apply_self_layout(base, req.as_ref());

    let Some(text) = widget.as_any().downcast_ref::<floom_widgets::Text>() else {
        return (style, None);
    };
    if pinned_width && pinned_height {
        return (style, None);
    }
    let state = Arc::new(Mutex::new(TextMeasure {
        text: text.content().to_string(),
        font_size: text.font_size(),
        weight: text.font_weight(),
        wrap_width: 0.0,
    }));
    if !pinned_width {
        style.width = None;
    }
    if !pinned_height {
        style.height = None;
    }
    (style, Some(state))
}

impl WidgetHost {
    pub fn new() -> Self {
        let mut focus = FocusManager::new();
        let scope = focus.create_scope(None);
        Self {
            nodes: Vec::new(),
            root: None,
            ids: IdAllocator::new(),
            router: EventRouter::new(),
            layout: LayoutEngine::new(),
            scene: Scene::new(),
            resources: ResourceRegistry::new(),
            glyph_cache: HashMap::new(),
            styled_glyph_cache: HashMap::new(),
            shaper: TextShaper::new(),
            text_layout_cache: LayoutCache::new(),
            pending_scene_removals: Vec::new(),
            semantics: SemanticsTree::new(),
            semantics_dirty: true,
            semantics_stale: Vec::new(),
            // Reused across layout passes so a re-layout does not allocate a
            // fresh `Vec<Option<Rect>>` per pass (grown to node count below).
            layout_relative: Vec::new(),
            layout_absolute: Vec::new(),
            painted_absolute: Vec::new(),
            content_stack: Vec::new(),
            scroll_inherited: Vec::new(),
            paused: false,
            focus,
            scope,
            keyboard_navigation: false,
            overlay: OverlayManager::new(),
            overlays: Vec::new(),
            overlay_commands: Vec::new(),
            next_overlay_handle: 0,
            ime_caret: None,
            anim_clock: 0.0,
            overlay_animating: false,
            native_anim: AnimationScheduler::new(),
            motion: AnimationScheduler::new(),
            node_anims: Vec::new(),
            motion_samples: Vec::new(),
            finished_declared: Vec::new(),
            native: None,
            participants: Vec::new(),
            frozen: Vec::new(),
            native_pool: NativeViewPool::new(),
            next_native_key: 0,
            scale: 1.0,
            native_focus: None,
            native_focus_return: None,
            pending_native_clears: Vec::new(),
            size: Size::new(800.0, 600.0),
            layout_dirty: true,
            content_dirty: false,
            last_stats: FrameStats::default(),
        }
    }

    /// Caps how many shaped-text results the paint path retains.
    ///
    /// Defaults to [`floom_text::DEFAULT_LAYOUT_CACHE_CAPACITY`]. Lower it for a
    /// memory-tight target, raise it for a screen with very many labels; the
    /// cache evicts least-recently-used entries once it is full, so the value
    /// is a ceiling rather than a preallocation.
    pub fn set_text_layout_cache_capacity(&mut self, capacity: usize) {
        self.text_layout_cache.set_capacity(capacity);
    }

    // --- tree construction -------------------------------------------------

    pub fn mount_root(&mut self, style: LayoutStyle, widget: impl WidgetAny + 'static) -> NodeId {
        let index = self.push(None, style, None, Box::new(widget));
        self.root = Some(index);
        self.semantics_dirty = true;
        NodeId(index)
    }

    pub fn mount(
        &mut self,
        parent: NodeId,
        style: LayoutStyle,
        widget: impl WidgetAny + 'static,
    ) -> NodeId {
        assert!(parent.0 < self.nodes.len(), "unknown parent node");
        self.semantics_dirty = true;
        NodeId(self.push(Some(parent.0), style, None, Box::new(widget)))
    }

    /// Takes a subtree out of the tree.
    ///
    /// The counterpart of [`WidgetHost::mount`]: an embedder mounting nodes by
    /// hand needs to undo it, and without this the only way to remove a node is
    /// to be a composite and stop declaring it. Layout, hit-test, focus and
    /// scene registrations all go with it.
    pub fn unmount(&mut self, node: NodeId) {
        if node.0 >= self.nodes.len() {
            return;
        }
        self.unmount_subtree(node.0);
        // A retired subtree is a change the frame loop has to be told about, or
        // a runner keying off `needs_frame` would leave it on screen.
        self.content_dirty = true;
        self.layout_dirty = true;
        self.semantics_dirty = true;
    }

    // --- overlays ----------------------------------------------------------

    /// Opens `content` as an overlay above the main tree, escaping its parent's
    /// clip so popups, dialogs and sheets can actually be seen and clicked.
    ///
    /// The content is laid out independently against the viewport at `bounds`,
    /// painted after (and therefore on top of) every main-tree node, and given
    /// hit-test priority — a modal overlay also swallows clicks that fall
    /// outside its content (the scrim). Focus moves into the content on open
    /// and is restored to whatever held it before on close.
    ///
    /// This is the host-side door, for an embedder mounting a dialog by hand.
    /// A widget asks through [`EventContext::open_overlay`] instead.
    ///
    /// [`EventContext::open_overlay`]: floom_core::EventContext::open_overlay
    pub fn open_overlay(
        &mut self,
        kind: OverlayKind,
        modal: bool,
        bounds: Rect,
        content: impl WidgetAny + 'static,
    ) -> OverlayHandle {
        let handle = OverlayHandle::new(self.next_overlay_handle);
        self.next_overlay_handle += 1;
        self.open_overlay_content(handle, None, kind, modal, bounds, Box::new(content))
    }

    /// Mounts a finished overlay content box under a handle the caller minted.
    ///
    /// The handle arrives from above rather than being made here because the
    /// content already has it: a widget's request builds its content with the
    /// handle in hand, and the overlay must be registered under that same one
    /// or the content's `close_overlay` would name an overlay that does not
    /// exist.
    fn open_overlay_content(
        &mut self,
        handle: OverlayHandle,
        owner: Option<ElementId>,
        kind: OverlayKind,
        modal: bool,
        bounds: Rect,
        content: Box<dyn WidgetAny>,
    ) -> OverlayHandle {
        let index = self.push(None, LayoutStyle::default(), None, content);
        self.nodes[index].is_overlay = true;
        // Reconcile the content immediately so a composite (e.g. a popup list)
        // mounts its children before this frame's layout pass.
        self.reconcile_subtree(index);
        // The entry animation starts on the host's clock, so an overlay opened
        // mid-frame and one opened from an event both begin from the same
        // reference the ticker advances.
        let now = self.anim_clock;
        let id = self
            .overlay
            .push(handle, kind, self.nodes[index].id, bounds, modal, now);
        let previous_focus = self.focus.focused();
        self.set_focus(self.nodes[index].id);
        self.overlays.push(OverlaySlot {
            id,
            node: index,
            bounds,
            previous_focus,
            owner,
        });
        // Tell the opener which handle it got. It is the only chance: the
        // request went into a queue and `open_overlay` could only answer "yes".
        if let Some(owner) = owner
            && let Some(owner_index) = self.index_of(owner)
        {
            let owner_id = self.nodes[owner_index].id;
            let mut cx = HostUpdateContext::callback(
                self.nodes[owner_index].bounds,
                Rect::from_origin_size(Point::ZERO, self.size),
            );
            self.nodes[owner_index]
                .widget
                .overlay_opened(&mut cx, owner_id, handle);
            let HostUpdateContext {
                dirty,
                children_dirty,
                ..
            } = cx;
            if children_dirty {
                self.nodes[owner_index].children_dirty = true;
            }
            if dirty {
                self.nodes[owner_index].dirty = true;
            }
        }
        self.content_dirty = true;
        // An overlay opened: its subtree joins the semantics tree.
        self.semantics_dirty = true;
        id
    }

    /// Closes and unmounts the overlay `id`, restoring focus to whatever held it
    /// before the overlay opened and handing the content back to whoever opened
    /// it.
    pub fn close_overlay(&mut self, id: OverlayHandle) {
        let Some(pos) = self.overlays.iter().position(|o| o.id == id) else {
            return;
        };
        let slot = self.overlays.remove(pos);
        match slot.previous_focus {
            Some(focused) => self.set_focus(focused),
            None => {
                self.clear_focus_flag(self.nodes[slot.node].id);
                self.focus.blur();
            }
        }
        // Take the content out before unmounting: the opener is about to be
        // given it, and a dead node holding the only box would strand it.
        let content = std::mem::replace(&mut self.nodes[slot.node].widget, Box::new(Retired));
        self.unmount_subtree(slot.node);
        self.overlay.remove(id);

        if let Some(owner) = slot.owner
            && let Some(index) = self.index_of(owner)
        {
            let owner_id = self.nodes[index].id;
            let mut cx = HostUpdateContext::callback(
                self.nodes[index].bounds,
                Rect::from_origin_size(Point::ZERO, self.size),
            );
            self.nodes[index]
                .widget
                .overlay_closed(&mut cx, owner_id, id, content);
            let HostUpdateContext {
                dirty,
                children_dirty,
                ..
            } = cx;
            if children_dirty {
                self.nodes[index].children_dirty = true;
                self.reconcile_subtree(index);
            }
            if dirty {
                self.nodes[index].dirty = true;
            }
            self.content_dirty = true;
            // An overlay closed: its subtree leaves the semantics tree.
            self.semantics_dirty = true;
        }
    }

    /// Number of overlays currently open.
    pub fn overlay_count(&self) -> usize {
        self.overlays.len()
    }

    /// Whether `id` is still an open overlay.
    pub fn has_overlay(&self, id: OverlayHandle) -> bool {
        self.overlays.iter().any(|o| o.id == id)
    }

    /// The viewport-relative bounds an open overlay is laid out against.
    ///
    /// `None` for a handle that names no open overlay. A getter rather than a
    /// public field because the overlay stack is z-ordered and renumbered as
    /// overlays open and close, so a caller should ask by handle rather than
    /// index into it.
    pub fn overlay_bounds(&self, id: OverlayHandle) -> Rect {
        self.overlays
            .iter()
            .find(|o| o.id == id)
            .map(|o| o.bounds)
            .unwrap_or_default()
    }

    /// The widget an open overlay mounted, by handle.
    ///
    /// The counterpart of [`Self::overlay_bounds`]: the two together are what a
    /// caller needs to say anything about an overlay it opened — where it is,
    /// and what it is. `None` for a handle that names no open overlay.
    pub fn overlay_widget(&self, id: OverlayHandle) -> Option<&dyn WidgetAny> {
        let slot = self.overlays.iter().find(|o| o.id == id)?;
        self.nodes.get(slot.node).map(|n| n.widget.as_ref())
    }

    /// The tree node an open overlay's content is mounted at, by handle.
    ///
    /// The door for retargeting a mounted overlay in place: a highlight box that
    /// has to follow the pointer cannot be rebuilt every frame — it is the same
    /// widget with a different rect — and [`Self::widget_mut`] needs a node.
    /// `None` for a handle that names no open overlay.
    ///
    /// Named `overlay_node_id` rather than `overlay_node` because a private
    /// helper of that name already exists and returns the raw index.
    pub fn overlay_node_id(&self, id: OverlayHandle) -> Option<NodeId> {
        let slot = self.overlays.iter().find(|o| o.id == id)?;
        self.nodes.get(slot.node).map(|_| NodeId(slot.node))
    }

    fn overlay_node(&self, id: OverlayHandle) -> Option<usize> {
        self.overlays.iter().find(|o| o.id == id).map(|o| o.node)
    }

    /// True when `index` sits under an overlay root (by ancestry), so the main
    /// paint/layout passes should leave it to the overlay-specific ones.
    fn is_in_overlay(&self, mut index: usize) -> bool {
        while let Some(parent) = self.nodes[index].parent {
            if self.nodes[parent].is_overlay {
                return true;
            }
            index = parent;
        }
        false
    }

    /// Lays out every open overlay independently against the viewport and
    /// positions the result at its `bounds` origin, writing hit regions with a
    /// z above the main tree so overlays always win a contest.
    fn layout_overlays(&mut self) {
        const OVERLAY_Z_BASE: i32 = 1_000_000_000;
        let slots: Vec<(usize, Rect, OverlayHandle)> = self
            .overlays
            .iter()
            .map(|s| (s.node, s.bounds, s.id))
            .collect();
        for (index, bounds, id) in slots {
            // Start from the node's own style and container intent, exactly as
            // `run_layout` does for the root. An overlay root is usually a
            // composite — a dialog's centred card, a sheet's bottom-aligned
            // column — whose axis and alignment come from its `LayoutRequest`;
            // laying it out from a bare default style would drop those and leave
            // its children stacked in the corner.
            let mut style = self.nodes[index].style.clone();
            if let Some(req) = self.nodes[index].widget.layout_request() {
                style = apply_self_layout(style, Some(&req));
            }
            style.width = Some(Length::Px(bounds.width() as f32));
            style.height = Some(Length::Px(bounds.height() as f32));
            self.layout.set_style(self.nodes[index].layout_node, style);
            self.layout
                .compute(self.nodes[index].layout_node, bounds.size());
            self.translate_subtree(index, bounds.x0, bounds.y0, OVERLAY_Z_BASE);
            self.overlay.set_bounds(id, self.nodes[index].bounds);
        }
    }

    /// Walks a subtree, offsetting every node's `bounds` by `(dx, dy)` (the
    /// overlay's viewport origin) and registering a high-z hit region so it
    /// participates in hit-testing on top of the main tree.
    fn translate_subtree(&mut self, root: usize, dx: f64, dy: f64, z_base: i32) {
        let mut stack = vec![root];
        while let Some(index) = stack.pop() {
            if self.nodes[index].dead {
                continue;
            }
            for &child in &self.nodes[index].children {
                if !self.nodes[child].dead {
                    stack.push(child);
                }
            }
            let out = self.layout.layout(self.nodes[index].layout_node);
            let r = out.rect;
            let bounds = Rect::from_origin_size(Point::new(r.x0 + dx, r.y0 + dy), r.size());
            self.nodes[index].bounds = bounds;
            let z = z_base + (self.nodes.len() as i32 - index as i32);
            self.router.hits.set_region(
                self.nodes[index].id,
                HitRegion {
                    bounds,
                    native: None,
                    z,
                },
            );
        }
    }

    /// Deepest overlay node containing `point`, so a click inside an overlay
    /// lands on the right child rather than the overlay root.
    fn hit_test_overlay_subtree(&self, root: usize, point: Point) -> Option<ElementId> {
        let mut best: Option<(usize, u32)> = None;
        let mut stack = vec![root];
        while let Some(index) = stack.pop() {
            if self.nodes[index].dead {
                continue;
            }
            for &child in &self.nodes[index].children {
                if !self.nodes[child].dead {
                    stack.push(child);
                }
            }
            if self.nodes[index].bounds.contains(point) {
                match best {
                    None => best = Some((index, self.nodes[index].depth)),
                    Some((_, depth)) if self.nodes[index].depth >= depth => {
                        best = Some((index, self.nodes[index].depth))
                    }
                    _ => {}
                }
            }
        }
        best.map(|(index, _)| self.nodes[index].id)
    }

    /// Builds and mounts one composite child, applying `parent_request` (the
    /// parent's [`LayoutRequest`] for this slot) plus the child's own container
    /// intent from [`Widget::layout_request`].
    fn mount_child(&mut self, parent_index: usize, child: &Child) -> usize {
        let widget = child.build();
        self.push(
            Some(parent_index),
            LayoutStyle::default(),
            Some(child.layout),
            widget,
        )
    }

    fn push(
        &mut self,
        parent: Option<usize>,
        style: LayoutStyle,
        parent_request: Option<LayoutRequest>,
        widget: Box<dyn WidgetAny>,
    ) -> usize {
        // Apply the widget's own container intent (flex axis, gap, absolute
        // position) on top of the supplied sizing. Composite children carry a
        // `parent_request` instead and are refreshed during reconciliation.
        let (style, text_measure) = match &parent_request {
            Some(req) => self_layout_with_text_measure(request_to_style(req), widget.as_ref()),
            None => self_layout_with_text_measure(style, widget.as_ref()),
        };
        let index = self.nodes.len();
        let id = {
            let mut cx = HostCreateContext {
                allocator: &mut self.ids,
            };
            widget.create(&mut cx)
        };
        let layout_node = match &text_measure {
            Some(state) => {
                let state = Arc::clone(state);
                let measure: MeasureFn = Box::new(move |avail| {
                    let mut m = state.lock().unwrap();
                    if m.text.is_empty() || m.font_size <= 0.0 {
                        return Size::ZERO;
                    }
                    // `avail.width` is the box this leaf actually landed in —
                    // wrap against it; `f64::INFINITY` (unconstrained) falls
                    // through to the unwrapped extent.
                    let measured = floom_text::measure_weighted_text(
                        &m.text,
                        m.font_size,
                        m.weight,
                        avail.width,
                    );
                    // Published for the paint path, which re-shapes this same string and must
                    // wrap it against the width it was measured at, not the rounded box.
                    m.wrap_width = avail.width;
                    measured
                });
                self.layout.new_leaf_with_measure(style.clone(), measure)
            }
            None => self.layout.new_leaf(style.clone()),
        };
        let mut depth = 0;
        if let Some(parent) = parent {
            let parent_node = self.nodes[parent].layout_node;
            self.layout.add_child(parent_node, layout_node);
            self.nodes[parent].children.push(index);
            self.router.set_parent(id, self.nodes[parent].id);
            depth = self.nodes[parent].depth + 1;
        }

        let focusable = !matches!(
            widget.semantics_hint().map(|hint| hint.role),
            None | Some(SemanticsRole::None)
        );
        if focusable {
            self.focus.register(self.scope, id);
        }

        // Deeper nodes win ties, and later siblings beat earlier ones on the
        // same level. Without this a full-viewport root would draw with a
        // button inside it, since equal `z` leaves the winner arbitrary.
        let z = depth as i32 * 1_000_000 + index as i32;
        self.router.hits.set_region(
            id,
            HitRegion {
                bounds: Rect::ZERO,
                native: None,
                z,
            },
        );
        self.nodes.push(Node {
            id,
            widget,
            layout_node,
            bounds: Rect::ZERO,
            parent,
            children: Vec::new(),
            child_keys: Vec::new(),
            style,
            parent_request,
            depth,
            scroll_offset: ScrollOffset::ZERO,
            dirty: true,
            children_dirty: true,
            dead: false,
            is_overlay: false,
            native: None,
            native_hidden: false,
            scene_nodes: Vec::new(),
            clip_push: None,
            clip_pop: None,
            group_push: None,
            group_pop: None,
            filter_push: None,
            filter_pop: None,
            text_measure,
            motion: None,
        });
        // A widget that animates from birth — an indeterminate progress bar —
        // declares its motion on the frame it is mounted. Waiting for it to
        // become dirty would leave it parked until something else changed it.
        self.sync_declared_animation(index);
        self.layout_dirty = true;
        index
    }

    /// Borrows the concrete widget behind `node`.
    pub fn widget<T: WidgetAny + 'static>(&self, node: NodeId) -> Option<&T> {
        self.nodes.get(node.0)?.widget.as_any().downcast_ref::<T>()
    }

    // --- composition -------------------------------------------------------

    /// Rebuilds the whole composite tree: for every live node whose children
    /// changed, diff `children()` by [`ChildKey`] and mount/unmount as needed,
    /// recursing into the result. Cheap for static composites because only
    /// `children_dirty` nodes are revisited.
    pub fn reconcile_all(&mut self) {
        if let Some(root) = self.root {
            self.reconcile_subtree(root);
        }
    }

    fn reconcile_subtree(&mut self, index: usize) {
        if self.nodes[index].dead {
            return;
        }
        if self.nodes[index].children_dirty {
            self.reconcile(index);
            self.nodes[index].children_dirty = false;
        }
        // Take the child list out instead of cloning it: this walk runs every
        // frame, and a clone would allocate once per node-with-children per
        // frame even when nothing changed. The recursion below never touches
        // *this* node's child list (a descendant's `reconcile` rewrites the
        // descendant's own list), so restoring the taken list afterwards is
        // faithful.
        let children = std::mem::take(&mut self.nodes[index].children);
        for child in &children {
            self.reconcile_subtree(*child);
        }
        self.nodes[index].children = children;
    }

    /// Reconciles `parent_index`'s direct children against `children()`, keyed
    /// by [`ChildKey`] so surviving widgets keep their state. Keys that
    /// disappear unmount their entire subtree; keys that appear build once.
    fn reconcile(&mut self, parent_index: usize) {
        let declared = self.nodes[parent_index].widget.children();
        // A node that declares no children and has never been reconciled as a
        // composite just keeps whatever children were mounted imperatively (via
        // `host.mount`). Only a node that *has* been reconciled before — its
        // `child_keys` is non-empty — means "no children now" as a diff against a
        // previous non-empty set, which is what should unmount the survivors.
        if declared.is_empty() && self.nodes[parent_index].child_keys.is_empty() {
            return;
        }

        let mut existing: HashMap<ChildKey, usize> = HashMap::new();
        for (&key, &child_idx) in self.nodes[parent_index]
            .child_keys
            .iter()
            .zip(self.nodes[parent_index].children.iter())
        {
            if !self.nodes[child_idx].dead {
                existing.insert(key, child_idx);
            }
        }

        let declared_keys: HashSet<ChildKey> = declared.iter().map(|child| child.key).collect();

        let mut new_children: Vec<usize> = Vec::with_capacity(declared.len());
        let mut new_keys: Vec<ChildKey> = Vec::with_capacity(declared.len());
        for child in &declared {
            let node_idx = if let Some(&idx) = existing.get(&child.key) {
                idx
            } else {
                self.mount_child(parent_index, child)
            };
            // Refresh the child's style in case its own container intent changed.
            // Measure-driven text leaves keep their auto dims: re-applying the
            // widget's `layout_request` here would freeze the mount-time extent
            // back into the style and defeat the measure callback.
            let style = {
                let node = &self.nodes[node_idx];
                let mut style = match node.parent_request.as_ref() {
                    Some(req) => apply_self_layout(
                        request_to_style(req),
                        node.widget.layout_request().as_ref(),
                    ),
                    None => {
                        apply_self_layout(node.style.clone(), node.widget.layout_request().as_ref())
                    }
                };
                if node.text_measure.is_some() {
                    // The dims owned by the measure fn were stored as `None`;
                    // anything the request just filled must be cleared again.
                    if node.style.width.is_none() {
                        style.width = None;
                    }
                    if node.style.height.is_none() {
                        style.height = None;
                    }
                }
                style
            };
            self.nodes[node_idx].style = style.clone();
            self.layout
                .set_style(self.nodes[node_idx].layout_node, style);
            new_children.push(node_idx);
            new_keys.push(child.key);
        }

        // Whatever was mounted under a key no longer declared is gone.
        for (&key, &idx) in &existing {
            if !declared_keys.contains(&key) {
                self.unmount_subtree(idx);
            }
        }

        self.nodes[parent_index].children = new_children;
        self.nodes[parent_index].child_keys = new_keys;

        // Push the key-ordered children into the engine in one shot so taffy
        // keeps the visual order we just computed.
        let child_layouts: Vec<LayoutNode> = self.nodes[parent_index]
            .children
            .iter()
            .map(|&i| self.nodes[i].layout_node)
            .collect();
        let parent_layout = self.nodes[parent_index].layout_node;
        self.layout.set_children(parent_layout, &child_layouts);
        self.layout_dirty = true;
        // The tree's shape (and therefore the semantics tree) changed.
        self.semantics_dirty = true;
    }

    /// Marks `index` and its whole subtree dead, releasing layout, hit-test and
    /// focus registrations. The node's `NodeId` stays valid (we never shift the
    /// vector), so other live nodes' references are unaffected.
    fn unmount_subtree(&mut self, index: usize) {
        if self.nodes[index].dead {
            return;
        }
        // A retired subtree leaves the semantics tree.
        self.semantics_dirty = true;
        // Detach from the parent's child list *before* marking dead so the parent
        // does not keep routing to a removed slot.
        if let Some(parent) = self.nodes[index].parent {
            let parent_node = &mut self.nodes[parent];
            if let Some(pos) = parent_node.children.iter().position(|&c| c == index) {
                parent_node.children.remove(pos);
                // Only a node reconciled as a composite has a key per child;
                // one that had children mounted into it imperatively (via
                // `WidgetHost::mount`) has none, and blindly removing at `pos`
                // panics on the mismatch.
                if pos < parent_node.child_keys.len() {
                    parent_node.child_keys.remove(pos);
                }
            }
        }

        let mut stack = vec![index];
        let mut removed: Vec<usize> = Vec::new();
        while let Some(i) = stack.pop() {
            if self.nodes[i].dead {
                continue;
            }
            for &c in &self.nodes[i].children {
                if !self.nodes[c].dead {
                    stack.push(c);
                }
            }
            removed.push(i);
        }
        for &i in &removed {
            let id = self.nodes[i].id;
            let layout = self.nodes[i].layout_node;
            self.layout.remove(layout);
            self.router.unparent(id);
            self.router.hits.remove(id);
            self.focus.unregister(id);
            // Scene nodes die at the next paint (their removal needs the
            // paint transaction); park them so nothing renders after now.
            self.pending_scene_removals
                .append(&mut self.nodes[i].scene_nodes);
            // An animation belongs to the node it runs on: a retired node keeps
            // none, or the scheduler would go on advancing a value nothing
            // reads and the frame loop would never park.
            self.retire_node_animations(i);
            self.nodes[i].dead = true;
        }
    }

    /// Mutably borrows the concrete widget behind `node`.
    ///
    /// Borrowing marks the tree dirty: the caller is about to reach a widget
    /// the frame loop cannot see into, so the next frame has to be scheduled
    /// even without a platform event. The mark is conservative — a borrow that
    /// ends up changing nothing costs one extra settling frame, and the frame
    /// loop clears it once it has run.
    pub fn widget_mut<T: WidgetAny + 'static>(&mut self, node: NodeId) -> Option<&mut T> {
        let widget = self
            .nodes
            .get_mut(node.0)?
            .widget
            .as_any_mut()
            .downcast_mut::<T>()?;
        self.content_dirty = true;
        // Out-of-band mutation can change a widget's semantics hint too; the
        // tree shape did not change, so one incremental refresh suffices.
        self.semantics_stale.push(node.0);
        Some(widget)
    }

    /// Whether another frame is needed: content changed outside the frame loop,
    /// or the last frame left work behind (a re-layout, unmounts that still owe
    /// the scene a node removal, or a scene that is still visibly changing).
    ///
    /// The frame loop drains all of these, so this settles to `false` once the
    /// tree stops moving — that is what makes content-driven redraw terminate.
    pub fn needs_frame(&self) -> bool {
        self.content_dirty
            || self.layout_dirty
            || self.overlay_animating
            || self.animations_running() > 0
            || !self.pending_scene_removals.is_empty()
            || !self.scene.damage().is_empty()
            // A repaint request that has not reached a frame yet is exactly
            // the "content changed outside the frame loop" case above — but
            // it lives in the channel, not on the host, so it is polled here.
            // `frame()` drains the channel, so this settles to `false` once
            // the request has been served.
            || floom_core::has_repaint_requests()
    }

    /// Advances the animation clock by `dt_secs` and re-samples every running
    /// overlay entry animation. Returns whether anything is still running.
    ///
    /// [`WidgetHost::frame`] calls this with a nominal frame step, which is
    /// what makes an entry animation deterministic and finite: 0.15 s at 60 Hz
    /// is nine frames, and the tenth finds nothing left to do. An embedder that
    /// measures its own frame delta (or a test that wants to jump to a specific
    /// point of the animation) can call it directly instead — the delta is
    /// relative, so repeated calls accumulate.
    pub fn tick_animations(&mut self, dt_secs: f64) -> bool {
        self.anim_clock += dt_secs.max(0.0);
        let overlays = self.overlay.tick_animations(self.anim_clock);
        let native = self.tick_native_transitions();
        let motion = self.tick_node_animations();
        self.overlay_animating = overlays || native;
        overlays || native || motion
    }

    // --- node animation -----------------------------------------------------
    //
    // Two doors in, one channel out. `animate` is the imperative one, for
    // anything that holds a host — an app, an example, a test. A widget's own
    // declaration ([`floom_core::Widget::animation`]) is the declarative one,
    // for the code that cannot hold a host: a widget is borrowed *out* of it
    // for the whole of every call the host makes into it, so it can only say
    // what it wants and let the host own what is running.
    //
    // Both end up in `node_anims` and both are read back through the node they
    // run on: the sampled compositor property is what the scene node is folded
    // with, and the sampled value is what the element's own paint reads.

    /// Starts `kind` on `node`, replacing whatever it was running.
    ///
    /// One animation per element. The compositor property a node carries is a
    /// single value, and driving two channels at once — a fade *and* a slide —
    /// would need it to be a composed node transform, which is the same
    /// missing piece that keeps `Scale` from working.
    ///
    /// The animation ends when its kind says so; an endless one runs until it
    /// is cancelled or its node is unmounted. Returns the scheduler's id,
    /// which is an opaque token: the host's own doors take a [`NodeId`], and
    /// they are what a caller wants.
    ///
    /// Note that this is the *imperative* owner. A node whose widget also
    /// declares motion is reconciled back to the declaration on its next
    /// update — the two owners are not combined, and a widget that animates
    /// itself should not also be animated from outside.
    pub fn animate(&mut self, node: NodeId, kind: AnimKind) -> AnimId {
        self.cancel_animation(node);
        let id = self.motion.start(kind, self.anim_clock);
        self.node_anims.push(NodeAnimation {
            node: node.0,
            id: Some(id),
            declared: None,
        });
        id
    }

    /// Redirects the animation on `node` toward `to`, keeping the value and
    /// the velocity it has right now.
    ///
    /// Returns whether the node had an animation to redirect. See
    /// [`AnimationScheduler::retarget`] for what "keeping the velocity" means:
    /// the animation becomes a spring, because a tween cannot be pointed
    /// somewhere else without either jumping its value or jumping its speed.
    pub fn retarget_animation(&mut self, node: NodeId, to: f64) -> bool {
        let Some(entry) = self.node_anims.iter().find(|entry| entry.node == node.0) else {
            return false;
        };
        let Some(id) = entry.id else {
            return false;
        };
        self.motion.retarget(id, to, self.anim_clock)
    }

    /// Stops the animation on `node`, if it has one, and releases the
    /// compositing it was applying.
    ///
    /// Returns whether there was one. A widget that declares its own motion
    /// will start it again on its next update, so this is for the imperative
    /// case; a declared one stops by the widget stopping declaring it.
    ///
    /// Stopping releases the channel, which makes this the deliberate opposite
    /// of an animation *finishing*. A finished animation leaves the node where
    /// it put it (that is what `tick_node_animations` parks the sample for)
    /// because its
    /// endpoint is a state the caller asked for; a stopped one is not, and
    /// leaving the node wrapped in a half-applied compositing would strand it
    /// in a look that no live animation is feeding and that only another
    /// animation could clear. So the node returns to what its widget paints,
    /// and the frame after this one is the frame that shows it.
    ///
    /// Redirecting rather than stopping is [`WidgetHost::retarget_animation`],
    /// which does carry the node's current look forward.
    pub fn cancel_animation(&mut self, node: NodeId) -> bool {
        let Some(position) = self
            .node_anims
            .iter()
            .position(|entry| entry.node == node.0)
        else {
            return false;
        };
        let entry = self.node_anims.remove(position);
        if let Some(id) = entry.id {
            self.motion.cancel(id);
        }
        self.nodes[node.0].motion = None;
        true
    }

    /// The sample parked on `node` this frame — the value its animation last
    /// wrote, whether or not that animation is still running.
    ///
    /// The host-side twin of `PaintContext::animation_value`: same number, for
    /// anyone reading from outside the widget rather than inside its paint. It
    /// outlives the animation that wrote it, which is what makes "where did
    /// the fade end up" answerable after the fade is over; stopping an
    /// animation (rather than letting it finish) is what clears it again.
    pub fn animation_value(&self, node: NodeId) -> Option<f64> {
        self.nodes
            .get(node.0)?
            .motion
            .as_ref()
            .map(|sample| sample.value)
    }

    /// How many node animations are running right now.
    ///
    /// Counts *running* ones: a declared animation that has finished is still
    /// remembered, so that its declaration does not look unfamiliar on the next
    /// reconcile, but it is not running and must not keep a frame loop awake.
    pub fn animations_running(&self) -> usize {
        self.node_anims
            .iter()
            .filter(|entry| entry.id.is_some())
            .count()
    }

    /// Brings the animation running on `index` in line with what its widget
    /// declares.
    ///
    /// Three outcomes: nothing declared and nothing running (the common case —
    /// one virtual call and out of the way), a declaration that changed
    /// (cancel and restart), and a declaration that went away (cancel). An
    /// *unchanged* declaration is left exactly as it is, which is what lets a
    /// widget re-state its motion on every update without restarting it every
    /// update.
    ///
    /// Only declarations are reconciled. "This widget declares nothing" and
    /// "this widget withdrew the declaration it had" are the same answer from
    /// the widget and must not be the same answer from the host: an entry this
    /// pass did not create is an imperative one (see [`WidgetHost::animate`]),
    /// and cancelling it here would mean that animating a node which then
    /// reports dirty — which is what a freshly mounted node does, and what any
    /// node does the moment its state changes — silently cancelled the
    /// animation it had just been asked to start.
    fn sync_declared_animation(&mut self, index: usize) {
        let declared = self.nodes[index].widget.animation();
        let position = self.node_anims.iter().position(|entry| entry.node == index);

        match (declared, position) {
            (None, None) => {}
            (None, Some(position)) => {
                if self.node_anims[position].declared.is_none() {
                    return;
                }
                let entry = self.node_anims.remove(position);
                if let Some(id) = entry.id {
                    self.motion.cancel(id);
                }
                self.nodes[index].motion = None;
            }
            (Some(spec), position) => {
                let unchanged = position.is_some_and(|position| {
                    self.node_anims[position]
                        .declared
                        .as_ref()
                        .is_some_and(|declared| declared.matches(&spec))
                });
                if unchanged {
                    return;
                }
                if let Some(position) = position {
                    let entry = self.node_anims.remove(position);
                    if let Some(id) = entry.id {
                        self.motion.cancel(id);
                    }
                }
                let kind = AnimKind::from(&spec);
                let id = self.motion.start(kind.clone(), self.anim_clock);
                self.node_anims.push(NodeAnimation {
                    node: index,
                    id: Some(id),
                    declared: Some(spec),
                });
                // Park the t = 0 sample *now*: a declared animation is started
                // here, after the frame's animation tick already ran, so without
                // this the node would paint the previous (settled) value on the
                // very frame its transition restarts — an implicit transition
                // would snap to its endpoint instead of starting from its origin.
                // The next `tick_node_animations` overwrites this with the real
                // first step, so it only closes the one-frame gap.
                let (value, props, _) = kind.sample(0.0);
                self.nodes[index].motion = Some(MotionSample { value, props });
            }
        }
    }

    /// Reconciles the declarations of the nodes that changed this frame.
    fn reconcile_declared_animations(&mut self, dirty: &[usize]) {
        for &index in dirty {
            if index < self.nodes.len() && !self.nodes[index].dead {
                self.sync_declared_animation(index);
            }
        }
    }

    /// Advances every node animation and parks this frame's sample on the node
    /// it runs on. Returns whether anything is still running.
    ///
    /// The list is walked back to front so that an entry can be removed while
    /// its neighbours keep their indices, and the samples live in a buffer the
    /// host owns rather than in a fresh vector per frame: a spinner that runs
    /// for a minute has hundreds of chances to be the reason a steady-state
    /// frame stopped being allocation-free.
    fn tick_node_animations(&mut self) -> bool {
        if self.animations_running() == 0 {
            return false;
        }
        let now = self.anim_clock;
        self.motion.tick_into(now, &mut self.motion_samples);

        // Declared animations that finished this tick: their widgets want the
        // event, so we tell them after the bookkeeping above — a widget's
        // `animation_finished` can mark itself dirty, and the update pass that
        // follows this in `frame` is where that mark takes effect. Reused
        // scratch rather than a fresh `Vec` so a settled frame allocates
        // nothing (see the steady-frame budget in B9).
        self.finished_declared.clear();

        for index in (0..self.node_anims.len()).rev() {
            let Some(id) = self.node_anims[index].id else {
                // A declared animation that already finished: its declaration
                // is still standing, so the entry stays put.
                continue;
            };
            let sampled = self
                .motion_samples
                .iter()
                .find(|sample| sample.id == id)
                .map(|sample| (sample.value, sample.compositor.clone(), sample.finished));

            match sampled {
                Some((value, props, finished)) => {
                    // Park the sample even on the last tick. An animation
                    // *leaves the node where it put it*: a fade that ends at
                    // zero opacity stays faded, and a slide that ends offset
                    // stays offset, until something else moves it. Clearing it
                    // here would snap the node back to its un-composited
                    // appearance on the very frame the motion finished, which
                    // is the one moment the seam would be most visible. (The
                    // overlay entry animation clears instead, because "no
                    // compositing" *is* the state it is entering — that is a
                    // different question, answered where it belongs.)
                    let node = self.node_anims[index].node;
                    self.nodes[node].motion = Some(MotionSample { value, props });
                    if finished {
                        let declared = self.node_anims[index].declared.is_some();
                        self.node_anims[index].id = None;
                        if !declared {
                            self.node_anims.remove(index);
                        } else {
                            self.finished_declared.push(node);
                        }
                    }
                }
                // Gone from the scheduler without the host retiring it: only
                // possible if something cancelled the id behind the host's
                // back. Nothing is running on the node now.
                None => {
                    let declared = self.node_anims[index].declared.is_some();
                    self.node_anims[index].id = None;
                    if !declared {
                        self.node_anims.remove(index);
                    }
                }
            }
        }

        for node in std::mem::take(&mut self.finished_declared) {
            self.notify_declared_motion_finished(node);
        }
        self.animations_running() > 0
    }

    /// Hands a declared animation's completion to its widget.
    ///
    /// Called from [`Self::tick_node_animations`] while the frame's clock has
    /// already advanced but the update pass has not yet run. The widget uses
    /// the event to do the one thing the host cannot decide for it — what a
    /// finished *presence* animation means: that an exiting subtree can now be
    /// unmounted, or an entering one has reached rest. The minimal context is
    /// enough: a presence widget reacts by re-marking itself dirty so the
    /// update pass folds the transition's end into the tree.
    fn notify_declared_motion_finished(&mut self, index: usize) {
        if self.nodes[index].dead {
            return;
        }
        let bounds = self.nodes[index].bounds;
        let viewport = Rect::from_origin_size(Point::ZERO, self.size);
        let mut cx = HostUpdateContext {
            dirty: false,
            children_dirty: false,
            layout_dirty: false,
            bounds,
            viewport,
            content_size: None,
            snapshot: None,
            overlays: None,
            native: None,
        };
        let id = self.nodes[index].id;
        self.nodes[index].widget.animation_finished(&mut cx, id);
        // The widget's reaction to "my motion ended" is to re-mark itself, so
        // the update pass this frame folds the transition's end into the tree
        // (an exiting presence drops its child, an entering one reaches rest).
        // The callback's context is throwaway, so its marks have to be folded
        // back into the node here — otherwise they would be silently dropped.
        if cx.dirty {
            self.nodes[index].dirty = true;
        }
        if cx.children_dirty {
            self.nodes[index].children_dirty = true;
        }
    }

    /// Drops whatever animation `index` had, running or merely remembered.
    fn retire_node_animations(&mut self, index: usize) {
        let mut position = self.node_anims.len();
        while position > 0 {
            position -= 1;
            if self.node_anims[position].node != index {
                continue;
            }
            if let Some(id) = self.node_anims[position].id {
                self.motion.cancel(id);
            }
            self.node_anims.remove(position);
            self.nodes[index].motion = None;
        }
    }

    /// The host's animation clock, in seconds since the first frame.
    pub fn anim_clock(&self) -> f64 {
        self.anim_clock
    }

    /// Whether any overlay is still playing its entry animation.
    pub fn overlay_animating(&self) -> bool {
        self.overlay_animating
    }

    /// Freezes or unfreezes non-essential work: the animation clock stops
    /// advancing (overlays, native-transition tweens) while the app is
    /// backgrounded or the OS has suspended it, and resumes from exactly the
    /// point it stopped. Frames still run — a paused host must still answer a
    /// platform-driven repaint with correct, settled pixels — they just make
    /// no animation progress while paused.
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    /// Whether animation progress is currently frozen.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn node_id(&self, node: NodeId) -> ElementId {
        self.nodes[node.0].id
    }

    /// The mounted root, if any.
    pub fn root(&self) -> Option<NodeId> {
        self.root.map(NodeId)
    }

    /// Handle for the `index`-th mounted node, in mount order.
    pub fn node_at(&self, index: usize) -> Option<NodeId> {
        (index < self.nodes.len()).then_some(NodeId(index))
    }

    /// The union of `index`'s descendants' boxes, relative to its own box, or
    /// `None` when it has no content.
    ///
    /// Measured from `layout_absolute`, the *unscrolled* position. A scrolling
    /// container's content does not change size when the container scrolls, and
    /// measuring the painted rect would make it look as though it did: the
    /// offset would shrink the content, which would clamp the offset, which
    /// would shrink it again.
    ///
    /// The node's own box is excluded. A scroll container's viewport is not its
    /// content, and including it would make a container whose child is smaller
    /// than itself report itself as the floor — right for nothing.
    ///
    /// Only ever called from a widget that asked, because a walk per node on a
    /// 100k-node tree is a cost paid for a question nobody asked. The stack is
    /// host-owned scratch, so the walk itself does not allocate.
    fn content_size(&mut self, index: usize) -> Option<Size> {
        // A node with no measured position cannot have a meaningful content
        // extent, and answering zero would be indistinguishable from empty.
        let Some(Some(own)) = self.layout_absolute.get(index) else {
            return None;
        };
        self.content_stack.clear();
        self.content_stack
            .extend(self.nodes[index].children.iter().copied());
        let mut union: Option<Rect> = None;
        while let Some(i) = self.content_stack.pop() {
            if self.nodes[i].dead {
                continue;
            }
            // An overlay escapes its parent's box by design, so its size is not
            // the content's.
            if self.nodes[i].is_overlay {
                continue;
            }
            self.content_stack
                .extend(self.nodes[i].children.iter().copied());
            let Some(&Some(rect)) = self.layout_absolute.get(i) else {
                continue;
            };
            union = Some(match union {
                None => rect,
                Some(u) => u.union(rect),
            });
        }
        // Relative to the node's own origin. Absolute union extents would make
        // the answer depend on where the widget sits on screen, so two identical
        // scroll containers at different points in a window would clamp
        // differently. The caller has its own bounds and can subtract them.
        let union = union?;
        Some(Size::new(
            (union.x1 - own.x0).max(0.0),
            (union.y1 - own.y0).max(0.0),
        ))
    }

    /// The `index`-th child of `node`, in declaration order.
    ///
    /// `None` for an unmounted node, and `None` when `index` is past the end —
    /// a caller walking a subtree cannot tell those apart from the handle alone,
    /// and a composite's child count changes with its own state.
    pub fn child_at(&self, node: NodeId, index: usize) -> Option<NodeId> {
        self.nodes
            .get(node.0)
            .and_then(|n| n.children.get(index))
            .map(|&i| NodeId(i))
    }

    /// How many children `node` has mounted right now.
    pub fn child_count(&self, node: NodeId) -> usize {
        self.nodes.get(node.0).map_or(0, |n| n.children.len())
    }

    /// The mounted node holding `id` — the inverse of [`Self::node_id`].
    pub fn node_for(&self, id: ElementId) -> Option<NodeId> {
        self.index_of(id).map(NodeId)
    }

    /// Whether `node` is still part of the tree.
    ///
    /// Unmounting does not free the slot — live `NodeId`s have to stay valid —
    /// so the slot keeps its widget and its box and only stops being walked.
    /// Anything that *searches* the tree therefore has to ask, or it will find
    /// a subtree that is no longer on screen.
    pub fn is_mounted(&self, node: NodeId) -> bool {
        self.nodes.get(node.0).is_some_and(|node| !node.dead)
    }

    /// Whether `node` still publishes `SemanticsHint` — mounted *and* its
    /// widget declares something. The pair is the common question, and getting
    /// it wrong is how a search ends up matching a retired node.
    pub fn live_semantics_hint(&self, node: NodeId) -> Option<SemanticsHint> {
        self.is_mounted(node)
            .then(|| self.semantics_hint(node))
            .flatten()
    }

    /// What this node announces to assistive technology, exactly as its widget
    /// declares it; `None` when the widget publishes nothing.
    ///
    /// [`Self::semantics`] is the lowered, hierarchy-aware tree that goes out
    /// to the platform. This is the raw declaration behind one of its entries —
    /// which is what a caller resolving a node by label or role wants, since it
    /// answers for the *widget* rather than for the tree's grouping of it.
    pub fn semantics_hint(&self, node: NodeId) -> Option<SemanticsHint> {
        self.nodes.get(node.0)?.widget.semantics_hint()
    }

    pub fn bounds(&self, node: NodeId) -> Rect {
        self.nodes[node.0].bounds
    }

    pub fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.nodes[node.0].parent.map(NodeId)
    }

    /// Restyles a mounted node and schedules a re-layout.
    pub fn set_style(&mut self, node: NodeId, style: LayoutStyle) {
        let index = node.0;
        self.layout
            .set_style(self.nodes[index].layout_node, style.clone());
        self.nodes[index].style = style;
        self.layout_dirty = true;
    }

    /// Scrolls `node`'s subtree by `offset`.
    ///
    /// The offset moves this node's descendants and not the node itself, so a
    /// scroll container's own frame stays put while its content moves — which
    /// is what makes it read as a window rather than as part of what it shows.
    ///
    /// Unlike [`Self::set_style`] this does not dirty the layout. A scroll
    /// offset is a post-layout transform on absolute coordinates, so re-running
    /// taffy would produce the same boxes and cost a pass for nothing. It does
    /// request a repaint, because the pixels move.
    ///
    /// Setting the same offset twice is free, so a widget can push its offset
    /// every frame without checking first.
    pub fn set_scroll_offset(&mut self, node: NodeId, offset: ScrollOffset) {
        let index = node.0;
        if self.nodes[index].scroll_offset == offset {
            return;
        }
        self.nodes[index].scroll_offset = offset;
        self.layout_dirty = true;
    }

    /// The scroll offset currently applied to `node`'s subtree.
    pub fn scroll_offset(&self, node: NodeId) -> ScrollOffset {
        self.nodes[node.0].scroll_offset
    }

    // --- viewport ----------------------------------------------------------

    pub fn set_size(&mut self, size: Size) {
        if self.size != size {
            self.size = size;
            self.layout_dirty = true;
        }
    }

    /// This host's tree, as plain data an inspector can describe into its own
    /// shape.
    ///
    /// The host's node table and a live tree are different data structures built
    /// for different jobs — one is laid out and painted, the other is queried —
    /// so a host that wants to be inspectable has to hand its nodes over as
    /// data. It hands over *data* rather than a live tree because the host sits
    /// at L7 and the inspector at L8: a host that built a live tree itself would
    /// depend upward, which `scripts/check-architecture-deps.sh` rejects and
    /// rightly so.
    ///
    /// The element kind is inferred rather than recorded, because the host does
    /// not store one: a widget with children is a host, a widget with none is a
    /// primitive. That is the same three-tier split the inspector reports, and
    /// it is the only thing the host honestly knows.
    ///
    /// Taken fresh on every call rather than cached: a panel that inspects a
    /// stale tree is worse than no panel, because it looks like an answer.
    pub fn tree_nodes(&self) -> Vec<HostedNode> {
        let mut out = Vec::new();
        // Depth-first in document order, so the tree reads the way it mounts.
        let mut stack = vec![self.root];
        while let Some(Some(index)) = stack.pop() {
            let Some(node) = self.nodes.get(index) else {
                continue;
            };
            out.push(HostedNode {
                key: format!("node-{}", node.id.raw()),
                kind: if node.children.is_empty() {
                    HostedKind::Primitive
                } else {
                    HostedKind::Host
                },
                element: node.id,
                parent: node.parent.map(|p| self.nodes[p].id),
            });
            // Reverse, so the first child is described first.
            for child in node.children.iter().rev() {
                stack.push(Some(*child));
            }
        }
        out
    }

    /// The viewport's logical size — what [`Self::set_size`] last set.
    ///
    /// A getter rather than a public field because the host's size is not just
    /// a number: the first thing a devtools panel needs to know is where the
    /// window's edge is, and an embedder asking for it should not have to reach
    /// into the tree to find out.
    pub fn size(&self) -> Size {
        self.size
    }

    /// Requests a re-layout on the next frame.
    pub fn invalidate_layout(&mut self) {
        self.layout_dirty = true;
    }

    // --- frame -------------------------------------------------------------

    /// Runs one full frame: update → layout → paint → semantics.
    pub fn frame(&mut self) -> FrameStats {
        let mut stats = FrameStats::default();

        // Advance the overlay animations first so update, layout and paint all
        // see the same sampled value — and so an overlay that opened since the
        // last frame is already a step into its entry rather than a frame of
        // nothing. A paused host (suspended/backgrounded embedder) freezes the
        // clock instead: no frame of animation progress, no resumed clock on
        // unpause — the resume picks up exactly where the pause left it.
        if !self.paused {
            self.tick_animations(NOMINAL_FRAME_SECS);
        }

        let mut retext = Vec::new();
        // Nodes whose widget reported dirty this frame: their semantics hint
        // may have changed with their content, so the semantics tree gets an
        // incremental upsert for exactly these elements (a full rebuild is
        // reserved for structural changes).
        let mut stale_semantics = Vec::new();
        let viewport = Rect::from_origin_size(Point::ZERO, self.size);
        // Overlay requests a widget makes from `update` — a composite flipping
        // `present` on a state change, with no event to react to — are queued
        // here and drained once the whole tree has had its turn, for exactly the
        // reason the event path queues them: opening one mounts a subtree and
        // needs the host to itself while the asking widget is borrowed out of it.
        let mut overlay_commands: Vec<OverlayCommand> = Vec::new();
        // Repaint requests that arrived outside the frame loop (a reactive
        // effect turning a signal change into `request_repaint`, say) fold
        // into the update pass below through the same dirty flag a widget's
        // own `update` reports — a repaint and nothing else. Taken once, up
        // front: a request repaints at most one frame, and requests for
        // elements that no longer exist simply match nothing.
        let repaint_requests: std::collections::HashSet<ElementId> =
            floom_core::take_repaint_requests().into_iter().collect();
        // Content extents, for the widgets that asked to be told — see
        // `Widget::needs_content_size`. Measured here, before the update pass
        // borrows any node, because a widget cannot reach the host to ask. Every
        // other node stays `None` and costs nothing, which is what keeps the walk
        // off the settled-frame path `tests/allocation.rs` budgets.
        let mut content_sizes: Vec<Option<Size>> = Vec::new();
        content_sizes.resize(self.nodes.len(), None);
        for (i, slot) in content_sizes.iter_mut().enumerate() {
            if !self.nodes[i].dead && self.nodes[i].widget.needs_content_size() {
                *slot = self.content_size(i);
            }
        }
        for index in 0..self.nodes.len() {
            if self.nodes[index].dead {
                continue;
            }
            // Read before the node is borrowed mutably: the update pass hands
            // the node out for the whole `update` call, and asking the host a
            // question from inside it is not possible.
            let snapshot = self.frozen_snapshot(index);
            let node = &mut self.nodes[index];
            let native = match self.native.as_mut() {
                Some(controller) => Some(NativeGrantSink {
                    node: index,
                    controller,
                    participants: &mut self.participants,
                    pool: &mut self.native_pool,
                    next_key: &mut self.next_native_key,
                    clears: &mut self.pending_native_clears,
                }),
                None => None,
            };
            let mut cx = HostUpdateContext {
                dirty: false,
                children_dirty: false,
                layout_dirty: false,
                bounds: node.bounds,
                viewport,
                content_size: content_sizes.get(index).copied().flatten(),
                snapshot,
                overlays: Some(UpdateOverlaySink {
                    commands: &mut overlay_commands,
                    next_handle: &mut self.next_overlay_handle,
                    owner: node.id,
                }),
                native,
            };
            node.widget.update(&mut cx, node.id);
            // A queued repaint request rides the same dirty flag a widget's
            // own `update` reports, so every downstream consumer (update
            // stats, semantics upsert, paint) treats the two identically.
            let dirty = cx.dirty || repaint_requests.contains(&node.id);
            node.dirty = dirty;
            node.children_dirty |= cx.children_dirty;
            // A widget that moved its own `layout_request` has to be able to say
            // so, or the frame would repaint the previous frame's geometry: a
            // padding or an intrinsic size would change nothing a user could see.
            if cx.layout_dirty {
                self.layout_dirty = true;
                stats.layout_requested += 1;
            }
            if dirty {
                stats.updated += 1;
                // A widget that changed may have changed its semantics hint
                // with it (labels, roles, states ride on widget state).
                stale_semantics.push(index);
                // Measure-driven text leaves must be re-measured before layout
                // when their content changed — otherwise the frozen extent
                // (and the stale glyph resource) survive another frame.
                if node.text_measure.is_some() {
                    retext.push(index);
                }
            }
        }
        // Widgets declare the motion they want on their own elements; the host
        // owns what is running. Reconciled exactly for the nodes that reported
        // dirty, because a declaration is a function of widget state and a
        // widget whose state changed is a widget that reported itself dirty —
        // which is what makes this pass cost nothing on a tree standing still.
        self.reconcile_declared_animations(&stale_semantics);
        self.semantics_stale = stale_semantics;
        // Drained *before* `reconcile_all`: a composite that just handed its
        // surface to a host overlay must have its in-tree children dropped in
        // the same frame, or the tree would paint the surface twice.
        self.drain_overlay_commands(overlay_commands);
        for index in retext {
            self.sync_text_measure(index);
        }

        // Build/mount/unmount composite subtrees before layout so newly added
        // nodes get sized and painted this frame.
        self.reconcile_all();

        if self.layout_dirty {
            self.run_layout();
            self.layout_dirty = false;
            stats.relaid = true;
        }
        // Overlays lay out against the viewport independently of the main tree.
        self.layout_overlays();
        stats.layout_nodes = self.layout.node_count();

        // Participants are positioned from the layout that just ran, and their
        // hit regions depend on it, so this is the earliest it can happen.
        self.sync_native(&mut stats);

        self.paint(&mut stats);

        // Semantics are refreshed only when something above may have changed
        // them: a structural change rebuilds the whole tree, and a content
        // change upserts just the elements whose widget reported dirty. A
        // settled frame skips the walk entirely instead of re-collecting a
        // tree that is byte-for-byte what it was.
        if self.semantics_dirty {
            self.collect_semantics();
            self.semantics_dirty = false;
            self.semantics_stale.clear();
        } else if !self.semantics_stale.is_empty() {
            for index in std::mem::take(&mut self.semantics_stale) {
                if self.nodes[index].dead {
                    continue;
                }
                self.update_semantics_node(index);
            }
        }

        // Everything the frame loop can observe has been consumed: the scene
        // carries this frame's changes, layout ran if it was marked, and any
        // out-of-band edits have been folded into the paint above.
        self.content_dirty = false;

        self.last_stats = stats;
        stats
    }

    /// Re-reads a text leaf's content into its measure state, invalidating
    /// layout when anything changed. Called only for nodes whose `update()`
    /// reported dirty, so the common static-text frame costs two string compares.
    fn sync_text_measure(&mut self, index: usize) {
        let Some(state) = &self.nodes[index].text_measure else {
            return;
        };
        let Some(text) = self.nodes[index]
            .widget
            .as_any()
            .downcast_ref::<floom_widgets::Text>()
        else {
            return;
        };
        let mut m = state.lock().unwrap();
        if m.text != text.content()
            || m.font_size != text.font_size()
            || m.weight != text.font_weight()
        {
            m.text = text.content().to_string();
            m.font_size = text.font_size();
            m.weight = text.font_weight();
            drop(m);
            // Taffy caches per-node layout and cannot observe the closure's
            // inputs changing — the node must be told explicitly.
            let layout_node = self.nodes[index].layout_node;
            self.layout.mark_dirty(layout_node);
            self.layout_dirty = true;
        }
    }

    fn run_layout(&mut self) {
        let Some(root) = self.root else {
            return;
        };
        let root_node = self.nodes[root].layout_node;
        if self.size.width <= 0.0 || self.size.height <= 0.0 {
            return;
        }
        // The root fills the viewport unless it asked for an explicit size, then
        // we apply the root widget's own container intent (e.g. a `Stack`'s
        // axis) on top. Without the fill, a `<Container>` root would shrink-wrap
        // its content and `justify_content` would have no free space to
        // distribute.
        //
        // The fill is deliberately *not* written back into `nodes[root].style`:
        // caching it would freeze the first frame's viewport into the stored
        // style (`width.is_none()` would stop holding) and later resizes could
        // never propagate. The stored style keeps whatever the app set; the
        // viewport fill is a per-pass override.
        let mut root_style = self.nodes[root].style.clone();
        if let Some(req) = self.nodes[root].widget.layout_request() {
            root_style = apply_self_layout(root_style, Some(&req));
        }
        if root_style.width.is_none() {
            root_style.width = Some(Length::Px(self.size.width as f32));
        }
        if root_style.height.is_none() {
            root_style.height = Some(Length::Px(self.size.height as f32));
        }
        self.layout.set_style(root_node, root_style);
        self.layout.compute(root_node, self.size);
        // Parent-relative and absolute rects, filled in the passes below.
        // Host-owned scratch, resized (not reallocated) per pass: a re-layout
        // at 100k nodes must not mint two fresh 100k-slot vectors to do it.
        self.layout_relative.clear();
        self.layout_relative.resize(self.nodes.len(), None);
        self.layout_absolute.clear();
        self.layout_absolute.resize(self.nodes.len(), None);
        self.painted_absolute.clear();
        self.painted_absolute.resize(self.nodes.len(), None);
        self.scroll_inherited.clear();
        self.scroll_inherited
            .resize(self.nodes.len(), ScrollOffset::ZERO);
        for index in 0..self.nodes.len() {
            if self.nodes[index].dead {
                continue;
            }
            // Overlays are laid out (and positioned) separately, against the
            // viewport, so the main-tree pass must not also size them.
            if self.nodes[index].is_overlay || self.is_in_overlay(index) {
                continue;
            }
            self.layout_relative[index] =
                Some(self.layout.layout(self.nodes[index].layout_node).rect);
        }
        // Taffy reports each node's location relative to its *parent*, so a
        // subtree whose ancestors are not at the window origin would paint and
        // hit-test shifted by exactly those offsets. Mount order puts every
        // parent before its children, so one forward pass accumulates each
        // node's absolute position out of its parent's.
        //
        // Scroll offsets accumulate in the same pass, for the same structural
        // reason: a node inherits its ancestors' positions, and it must inherit
        // their scroll offsets with them or a scroll container nested in another
        // would move only its own content. Two nested containers compose; a
        // container and its siblings stay independent, because a node's own
        // offset is applied to its children and never to itself.
        // `scroll_inherited` is the sum of the offsets of every *ancestor* — the
        // offset this node's subtree is displaced by. A node's own offset is
        // deliberately excluded: it displaces its children, not itself, which is
        // what keeps a scroll container's frame still while its content moves.
        for index in 0..self.nodes.len() {
            let Some(mut rect) = self.layout_relative[index] else {
                continue;
            };
            let mut inherited = ScrollOffset::ZERO;
            if let Some(parent) = self.nodes[index].parent {
                if let Some(parent_rect) = self.layout_absolute[parent] {
                    rect.x0 += parent_rect.x0;
                    rect.y0 += parent_rect.y0;
                    rect.x1 += parent_rect.x0;
                    rect.y1 += parent_rect.y0;
                }
                inherited = self.scroll_inherited[parent];
            }
            self.layout_absolute[index] = Some(rect);
            self.scroll_inherited[index] = inherited.then(self.nodes[index].scroll_offset);

            let dx = inherited.x;
            let dy = inherited.y;
            if dx != 0.0 || dy != 0.0 {
                rect.x0 += dx;
                rect.y0 += dy;
                rect.x1 += dx;
                rect.y1 += dy;
            }
            self.painted_absolute[index] = Some(rect);
        }
        for index in 0..self.nodes.len() {
            let Some(rect) = self.painted_absolute[index] else {
                continue;
            };
            self.nodes[index].bounds = rect;
            // A participant that is off screen has no region at all, not just
            // no native claim — see `Node::native_hidden`.
            if self.nodes[index].native_hidden {
                self.router.hits.remove(self.nodes[index].id);
                continue;
            }
            let z = self.nodes[index].depth as i32 * 1_000_000 + index as i32;
            self.router.hits.set_region(
                self.nodes[index].id,
                HitRegion {
                    bounds: rect,
                    // A node with a participant resolves to the native view,
                    // which takes the pointer itself. Set by the grant, read
                    // back here so a relayout keeps it.
                    native: self.nodes[index].native,
                    z,
                },
            );
        }
    }

    fn paint(&mut self, stats: &mut FrameStats) {
        // Damage reflects exactly this frame's changes: `Scene::upsert` filters
        // identical payloads, so an unchanged widget damages nothing and the
        // region only covers what actually moved, grew or vanished.
        self.scene.clear_damage();
        self.scene.begin_update();

        // Retire scene nodes orphaned by unmounts that happened between paint
        // passes (scene removal needs an active transaction).
        let orphans = std::mem::take(&mut self.pending_scene_removals);
        for id in orphans {
            self.scene.remove(id);
        }

        // Scene nodes parked on a node that died between two paints. Unmount
        // already pends these; draining defensively means a dead node can
        // never keep a phantom scene node alive.
        for index in 0..self.nodes.len() {
            if !self.nodes[index].dead || self.nodes[index].scene_nodes.is_empty() {
                continue;
            }
            for id in self.nodes[index].scene_nodes.drain(..) {
                self.scene.remove(id);
            }
        }

        // The main tree is walked depth-first from its roots rather than as one
        // flat sweep, so a node whose widget declares a clip can bracket its
        // own subtree with the push/pop markers. The two walks visit the same
        // nodes in the same order — a child is mounted immediately after its
        // parent, so ascending index *is* pre-order — and
        // `the_paint_walk_is_preorder` holds the invariant that makes it so.
        for index in 0..self.nodes.len() {
            if self.nodes[index].is_overlay || self.nodes[index].dead {
                continue;
            }
            // Overlay subtrees are painted afterwards, on top of the main tree.
            if self.nodes[index].parent.is_some() {
                continue;
            }
            self.paint_subtree(index, stats, SmallVec::new());
        }

        // Overlays paint last, so their content sits above everything beneath
        // it in z — this is what lets a popup escape its parent's clip. Each
        // carries the value its entry animation sampled this frame, which the
        // paint context folds into every node it writes.
        let mut overlay_nodes: Vec<(usize, SmallVec<[CompositorProp; 2]>)> = Vec::new();
        for slot in &self.overlays {
            if !self.nodes[slot.node].dead {
                overlay_nodes.push((
                    slot.node,
                    self.overlay.composite(slot.id).into_iter().collect(),
                ));
            }
        }
        for (node, composite) in overlay_nodes {
            self.paint_overlay_subtree(node, composite, stats);
        }
        self.scene.finish_update();
    }

    /// Paints one node into the scene, reusing its previous frame's scene nodes
    /// positionally (so unchanged payloads produce no damage) and retiring the
    /// ones it no longer produces.
    fn paint_node(
        &mut self,
        index: usize,
        inherited: SmallVec<[CompositorProp; 2]>,
        stats: &mut FrameStats,
    ) {
        // The node's id list is painted in place — no per-frame replacement
        // vector, so a settled frame does not allocate per node.
        let mut ids = std::mem::take(&mut self.nodes[index].scene_nodes);
        {
            let node = &self.nodes[index];
            // A node's own animation beats one it merely sits inside (an
            // overlay's entry): the more specific declaration is the one its
            // owner asked for.
            let motion = node.motion.clone();
            // Own channels win as a whole; an animation that drives nothing does
            // not crowd out an inherited overlay fade.
            let composite = match &motion {
                Some(sample) if !sample.props.is_empty() => sample.props.clone(),
                _ => inherited,
            };
            let mut cx = NodePaintContext {
                scene: &mut self.scene,
                resources: &mut self.resources,
                glyphs: &mut self.glyph_cache,
                styled_glyphs: &mut self.styled_glyph_cache,
                shaper: &mut self.shaper,
                layout_cache: &mut self.text_layout_cache,
                composite,
                animation: motion.as_ref().map(|sample| sample.value),
                wrap_width: node
                    .text_measure
                    .as_ref()
                    .and_then(|state| state.lock().ok())
                    .map_or(0.0, |m| m.wrap_width),
                prev: ids,
                used: 0,
                bounds: node.bounds,
            };
            node.widget.paint(&mut cx, node.id);
            ids = cx.finish();
        }
        self.nodes[index].scene_nodes = ids;
        stats.painted += 1;
    }

    /// Paints `index` and everything under it, bracketing the subtree with
    /// this node's clip markers when its widget declares a clip.
    ///
    /// A walk, rather than the flat sweep this used to be, for exactly one
    /// reason: the bracket. A widget's `paint` writes scene nodes for itself
    /// and nothing for descendants it does not paint, so only the host knows
    /// where a clipped subtree begins and ends. Nothing else changes — each
    /// Keeps only the *fold* class of a compositor property — opacity and the
    /// two translations, the ones that fold into a node's own payloads. Scale
    /// and rotate are deliberately dropped: they bracket the subtree via a
    /// group scope instead, and folding them per node would tear the picture
    /// apart (see [`Self::node_group_transform`]).
    fn fold_class_only(props: &[CompositorProp]) -> SmallVec<[CompositorProp; 2]> {
        props
            .iter()
            .filter(|p| {
                matches!(
                    p,
                    CompositorProp::Opacity(_)
                        | CompositorProp::TranslateX(_)
                        | CompositorProp::TranslateY(_)
                )
            })
            .copied()
            .collect()
    }

    /// Paints `index` and everything under it, bracketing the subtree with
    /// this node's clip markers when its widget declares a clip.
    ///
    /// A walk, rather than the flat sweep this used to be, for exactly one
    /// reason: the bracket. A widget's `paint` writes scene nodes for itself
    /// and nothing for descendants it does not paint, so only the host knows
    /// where a clipped subtree begins and ends. Nothing else changes — each
    /// node still paints at its own turn with its own composite.
    ///
    /// `inherited` is the fold-class composite (opacity / translate) an
    /// *ancestor's* presence animation is driving onto the whole subtree it
    /// wraps. A node's own motion beats an inherited one, so a descendant that
    /// animates its own opacity is dimmed by its own animation, not the
    /// ancestor's; one with no animation of its own inherits the ancestor's.
    fn paint_subtree(
        &mut self,
        index: usize,
        stats: &mut FrameStats,
        inherited: SmallVec<[CompositorProp; 2]>,
    ) {
        if self.nodes[index].dead {
            return;
        }
        let clip = self.node_clip_region(index);
        let clipped = clip.is_some();
        match clip {
            Some(region) => self.write_clip_push(index, region),
            // A widget that stopped declaring a clip must not leave its window
            // applied to the subtree it no longer covers.
            None => self.retire_clip_markers(index),
        }
        // The composite this node's own payloads and group scope see: its own
        // motion wins over a frozen native transition, which wins over an
        // inherited fold from an ancestor's presence animation.
        let own = self.nodes[index]
            .motion
            .as_ref()
            .map(|s| s.props.clone())
            .unwrap_or_default();
        let own_fold = Self::fold_class_only(&own);
        let frozen = self.frozen_composite(index);
        let composite = if !own.is_empty() {
            own
        } else if !frozen.is_empty() {
            frozen
        } else {
            inherited.clone()
        };
        // A transform-class composite (scale, rotate) cannot fold into the
        // node's own payloads — scaling each rect about its own centre would
        // tear the picture apart. It becomes a group scope bracketing the
        // subtree, pivoting about the node's box centre.
        let group = self.node_group_transform(index, &composite);
        let transformed = group.is_some();
        match group {
            Some(transform) => self.write_group_push(index, transform),
            None => self.retire_group_markers(index),
        }
        // A declared filter wraps the subtree: this widget's own paint *and*
        // every descendant's, because what the filter sees has to be the
        // composite of both. Inside the clip (a rounded card's blur stays
        // inside the card) and outside the group scope (the filter runs in the
        // space the content was laid out in, not in the rotated one).
        // An identity filter is "no filter": writing markers for it would
        // make every backend pay for a layer to do nothing, and a `FilterPop`
        // with no matching visual effect is one more thing that can go wrong.
        match self.nodes[index]
            .widget
            .filter()
            .filter(|f| !f.is_identity())
        {
            Some(filter) => self.write_filter_push(index, filter),
            None => self.retire_filter_markers(index),
        }
        self.paint_node(index, composite, stats);
        // The fold an ancestor's presence fade hands down: the node's own fold
        // class wins, so it does not double-apply; a transform-class own prop
        // does not fold, so the inherited fold still reaches descendants.
        let fold_down = if own_fold.is_empty() {
            inherited
        } else {
            own_fold
        };
        self.paint_children(index, stats, fold_down);
        if self.nodes[index].filter_push.is_some() {
            self.write_filter_pop(index);
        }
        if transformed {
            self.write_group_pop(index);
        }
        if clipped {
            self.write_clip_pop(index);
        }
    }

    /// Paints `index`'s children in order.
    ///
    /// Walked by index rather than collected into a `Vec` first: cloning the
    /// child list is one allocation per node per frame, which is what the
    /// settled-frame allocation budget exists to forbid. `inherited` is the
    /// fold-class composite to hand each child (see [`Self::paint_subtree`]).
    fn paint_children(
        &mut self,
        index: usize,
        stats: &mut FrameStats,
        inherited: SmallVec<[CompositorProp; 2]>,
    ) {
        let mut i = 0;
        while i < self.nodes[index].children.len() {
            let child = self.nodes[index].children[i];
            self.paint_subtree(child, stats, inherited.clone());
            i += 1;
        }
    }

    /// The clip this node's widget asks for, resolved against its laid-out box.
    ///
    /// `None` when the widget asks for nothing — or when it has no box yet
    /// (see [`resolve_clip`]), because clipping to an empty box would make the
    /// subtree invisible rather than the no-op "not laid out yet" means.
    fn node_clip_region(&self, index: usize) -> Option<ClipRegion> {
        let shape = self.nodes[index].widget.clip()?;
        resolve_clip(&shape, self.nodes[index].bounds)
    }

    /// Opens this node's clip: writes the `ClipPush` marker into the slot the
    /// node holds for it, minting the slot on the first frame the clip appears.
    ///
    /// *Where* the slot comes from is the whole trick. The scene draws in slot
    /// order, and a slot is placed where it is allocated; allocating here — on
    /// the way into the subtree, before the node's own scene nodes exist —
    /// puts the marker exactly on the subtree's leading edge. Keeping the id
    /// puts it there on every later frame, since `upsert` never moves a node.
    fn write_clip_push(&mut self, index: usize, region: ClipRegion) {
        let node = SceneNode::ClipPush { region };
        match self.nodes[index].clip_push {
            Some(id) => self.scene.upsert(id, node),
            None => {
                // Same mid-life ordering hazard as `write_group_push`: a clip
                // declared after the subtree has painted would slot after the
                // very picture it is supposed to bracket ahead of. Reseed
                // first, so the push claims the subtree's own old slot.
                self.reseed_subtree_scene_nodes(index);
                let id = self.scene.insert(node);
                self.nodes[index].clip_push = Some(id);
            }
        }
    }

    /// Closes the region [`Self::write_clip_push`] opened — after the last
    /// descendant has painted, so it lands on the subtree's trailing edge.
    fn write_clip_pop(&mut self, index: usize) {
        match self.nodes[index].clip_pop {
            Some(id) => self.scene.upsert(id, SceneNode::ClipPop),
            None => {
                let id = self.scene.insert(SceneNode::ClipPop);
                self.nodes[index].clip_pop = Some(id);
            }
        }
    }

    /// Retires both markers of a node that no longer declares a clip.
    fn retire_clip_markers(&mut self, index: usize) {
        if let Some(id) = self.nodes[index].clip_push.take() {
            self.scene.remove(id);
        }
        if let Some(id) = self.nodes[index].clip_pop.take() {
            self.scene.remove(id);
        }
    }

    /// The transform a transform-class composite asks for on this node, if any:
    /// the scale/rotate channels of the effective composite (already resolved
    /// by the caller: own animation wins over inherited) multiplied against
    /// the node's laid-out box, so the scope pivots about the box's centre.
    /// `None` when no channel asks for a transform — opacity and translate fold
    /// per node, and there is no laid-out box to pivot an empty one around.
    fn node_group_transform(
        &self,
        index: usize,
        composite: &[CompositorProp],
    ) -> Option<floom_types::Transform> {
        let bounds = self.nodes[index].bounds;
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return None;
        }
        let center = floom_types::Point::new(
            bounds.x0 + bounds.width() / 2.0,
            bounds.y0 + bounds.height() / 2.0,
        );
        let mut transform = None;
        for prop in composite {
            match *prop {
                CompositorProp::Scale(s) => {
                    transform = Some(compose_transform(transform, {
                        floom_types::Transform::scale_around(s, s, center)
                    }));
                }
                CompositorProp::Rotate(radians) => {
                    transform = Some(compose_transform(transform, {
                        floom_types::Transform::rotate_around(radians, center)
                    }));
                }
                _ => {}
            }
        }
        transform
    }

    /// Opens this node's transform scope, on the same retained-slot scheme as
    /// [`Self::write_clip_push`]: the marker is minted on the way into the
    /// node's picture and kept there, so later frames only `upsert` it.
    fn write_group_push(&mut self, index: usize, transform: floom_types::Transform) {
        let node = SceneNode::GroupPush { transform };
        match self.nodes[index].group_push {
            Some(id) => self.scene.upsert(id, node),
            None => {
                // The scope arrived mid-life: the subtree's picture already
                // holds earlier slots, and slot order is draw order — a push
                // appended now would land *after* the picture it must wrap.
                // Reseed first: the freed slots are reused oldest-first, so
                // the push takes the subtree's own old slot and every node
                // re-mints behind it.
                self.reseed_subtree_scene_nodes(index);
                let id = self.scene.insert(node);
                self.nodes[index].group_push = Some(id);
            }
        }
    }

    /// Drops the subtree's retained scene-node ids so the next paint of every
    /// node under `index` re-inserts after whatever was just minted here.
    /// One allocation-order fix per bracket appearance, not a per-frame cost.
    fn reseed_subtree_scene_nodes(&mut self, index: usize) {
        if self.nodes[index].dead {
            return;
        }
        for id in std::mem::take(&mut self.nodes[index].scene_nodes) {
            self.scene.remove(id);
        }
        let mut i = 0;
        while i < self.nodes[index].children.len() {
            let child = self.nodes[index].children[i];
            self.reseed_subtree_scene_nodes(child);
            i += 1;
        }
    }

    /// Closes the scope [`Self::write_group_push`] opened — after the last
    /// descendant has painted.
    fn write_group_pop(&mut self, index: usize) {
        match self.nodes[index].group_pop {
            Some(id) => self.scene.upsert(id, SceneNode::GroupPop),
            None => {
                let id = self.scene.insert(SceneNode::GroupPop);
                self.nodes[index].group_pop = Some(id);
            }
        }
    }

    /// Retires both markers of a node no longer animated by a transform.
    fn retire_group_markers(&mut self, index: usize) {
        if let Some(id) = self.nodes[index].group_push.take() {
            self.scene.remove(id);
        }
        if let Some(id) = self.nodes[index].group_pop.take() {
            self.scene.remove(id);
        }
    }

    /// Opens this node's filter scope, on the same retained-slot scheme as
    /// [`Self::write_group_push`] — including the mid-life reseed, which this
    /// one needs just as much: a card that gains a blur after its first paint
    /// would otherwise be slotted *after* the picture it has to filter.
    fn write_filter_push(&mut self, index: usize, filter: floom_types::Filter) {
        let node = SceneNode::FilterPush { filter };
        match self.nodes[index].filter_push {
            Some(id) => self.scene.upsert(id, node),
            None => {
                self.reseed_subtree_scene_nodes(index);
                let id = self.scene.insert(node);
                self.nodes[index].filter_push = Some(id);
            }
        }
    }

    /// Closes the scope [`Self::write_filter_push`] opened — after the last
    /// descendant has painted.
    fn write_filter_pop(&mut self, index: usize) {
        match self.nodes[index].filter_pop {
            Some(id) => self.scene.upsert(id, SceneNode::FilterPop),
            None => {
                let id = self.scene.insert(SceneNode::FilterPop);
                self.nodes[index].filter_pop = Some(id);
            }
        }
    }

    /// Retires both markers of a node that no longer declares a filter.
    ///
    /// Not optional: a widget that stops filtering must stop paying for a
    /// layer, and a `FilterPop` left behind would close some *other* scope —
    /// or none, silently.
    fn retire_filter_markers(&mut self, index: usize) {
        if let Some(id) = self.nodes[index].filter_push.take() {
            self.scene.remove(id);
        }
        if let Some(id) = self.nodes[index].filter_pop.take() {
            self.scene.remove(id);
        }
    }

    /// Pre-order paint of an overlay subtree (parent before its children),
    /// applying the overlay's entry animation to every node it writes.
    fn paint_overlay_subtree(
        &mut self,
        index: usize,
        composite: SmallVec<[CompositorProp; 2]>,
        stats: &mut FrameStats,
    ) {
        if self.nodes[index].dead {
            return;
        }
        // The effective composite: the node's own animation wins over the
        // overlay's entry animation (the same rule `paint_node` applies).
        let own = self.nodes[index]
            .motion
            .as_ref()
            .map(|s| s.props.clone())
            .unwrap_or_default();
        let composite = if !own.is_empty() { own } else { composite };
        // An overlay clips exactly the way a main-tree node does. It has to:
        // a popup whose content is clipped is the *reason* an overlay is
        // painted after its parent in the first place.
        let clip = self.node_clip_region(index);
        let clipped = clip.is_some();
        match clip {
            Some(region) => self.write_clip_push(index, region),
            None => self.retire_clip_markers(index),
        }
        // Transform scopes work under overlays too: the entry composite is
        // inherited, but a node's own scale/rotate animation still brackets.
        let group = self.node_group_transform(index, &composite);
        let transformed = group.is_some();
        match group {
            Some(transform) => self.write_group_push(index, transform),
            None => self.retire_group_markers(index),
        }
        // Filters work under overlays for the same reason the clip and the
        // group scope do: an overlay is painted after its parent, not painted
        // differently.
        // An identity filter is "no filter": writing markers for it would
        // make every backend pay for a layer to do nothing, and a `FilterPop`
        // with no matching visual effect is one more thing that can go wrong.
        match self.nodes[index]
            .widget
            .filter()
            .filter(|f| !f.is_identity())
        {
            Some(filter) => self.write_filter_push(index, filter),
            None => self.retire_filter_markers(index),
        }
        self.paint_node(index, composite.clone(), stats);
        let mut i = 0;
        while i < self.nodes[index].children.len() {
            let child = self.nodes[index].children[i];
            self.paint_overlay_subtree(child, composite.clone(), stats);
            i += 1;
        }
        if self.nodes[index].filter_push.is_some() {
            self.write_filter_pop(index);
        }
        if transformed {
            self.write_group_pop(index);
        }
        if clipped {
            self.write_clip_pop(index);
        }
    }

    /// Builds the semantics node for one mounted element, or `None` when the
    /// element publishes nothing (no hint of its own and no semantic
    /// descendants) and therefore has no entry in the tree.
    fn build_semantics_node(&self, index: usize) -> Option<SemanticsNode> {
        let node = &self.nodes[index];
        let hint = node.widget.semantics_hint();
        let publishes_itself = hint
            .as_ref()
            .is_some_and(|hint| hint.role != SemanticsRole::None || hint.label.is_some());
        let has_children = node.children.iter().any(|&child| !self.nodes[child].dead);
        if !publishes_itself && !has_children {
            return None;
        }

        let children: Vec<ElementId> = node
            .children
            .iter()
            .copied()
            .filter(|&child| !self.nodes[child].dead)
            .map(|child| self.nodes[child].id)
            .collect();

        // A purely structural node with semantic descendants becomes a
        // `Group` so the assistive-technology tree keeps its hierarchy.
        let role = match hint.as_ref().map(|hint| hint.role) {
            Some(SemanticsRole::None) => floom_accessibility::SemanticsRole::Group,
            Some(role) => to_a11y_role(role),
            None => floom_accessibility::SemanticsRole::Group,
        };
        let state = hint
            .as_ref()
            .map(|hint| to_a11y_state(hint.state))
            .unwrap_or_else(floom_accessibility::SemanticsState::empty);

        let mut semantics_node = SemanticsNode::new(role).with_state(state);
        if let Some(label) = hint.and_then(|hint| hint.label) {
            semantics_node = semantics_node.with_label(label);
        }
        for child in children {
            semantics_node = semantics_node.with_child(child);
        }
        Some(semantics_node)
    }

    fn collect_semantics(&mut self) {
        // Rebuild in place: clearing (not replacing) the tree keeps its map
        // capacity, so a rebuild allocates for structural content — a child
        // list, a label — rather than per element for storage.
        self.semantics.begin_rebuild();
        for index in 0..self.nodes.len() {
            if self.nodes[index].dead {
                continue;
            }
            if let Some(semantics_node) = self.build_semantics_node(index) {
                let id = self.nodes[index].id;
                self.semantics.set_node(id, semantics_node);
            }
        }
        if let Some(root) = self.root {
            self.semantics.set_root(self.nodes[root].id);
        }
    }

    /// Refreshes the semantics entry of one element whose *content* changed
    /// without any structural change (a renamed label, a new text, a new
    /// state). This is the incremental counterpart of
    /// [`Self::collect_semantics`]: a full rebuild walks every element and
    /// allocates per structural node, which a frame that changed one label
    /// has no business paying for.
    fn update_semantics_node(&mut self, index: usize) {
        let id = self.nodes[index].id;
        match self.build_semantics_node(index) {
            Some(semantics_node) => self.semantics.set_node(id, semantics_node),
            // The element stopped publishing (its only label vanished): drop
            // the entry instead of leaving a stale one behind.
            None => self.semantics.remove(id),
        }
    }

    // --- input -------------------------------------------------------------

    /// Routes `event` to the widget under the pointer (or the focused widget
    /// for key events) and returns whether any widget consumed it.
    pub fn dispatch(&mut self, event: Event) -> bool {
        // A press that misses the topmost overlay takes it down — the light
        // dismiss every menu and picker is expected to have. It is a host rule
        // rather than a per-widget one because a widget cannot see a press that
        // lands outside itself: the router never routes it there, so each popup
        // would have to invent a scrim it does not otherwise want. Consuming
        // the press is the point — the click that closes a menu must not also
        // activate whatever was under it.
        if self.dismiss_top_overlay_on_press(&event) {
            return true;
        }
        // Gate #7 (focus + basic keyboard navigation): Tab and Shift+Tab move
        // focus through the focusable tree. A navigation key carries no text, so
        // it is consumed here and never routed to a widget — routing it onward
        // would, for example, try to insert a tab into a focused text field.
        if let Event::Key(KeyEvent {
            phase: KeyPhase::Down,
            key: Key::Named(NamedKey::Tab),
            modifiers,
            ..
        }) = &event
        {
            if modifiers.contains(Modifiers::SHIFT) {
                self.focus_prev();
            } else {
                self.focus_next();
            }
            return true;
        }
        let target = match event {
            // Overlays win over the main tree, front-to-back; a modal overlay
            // also swallows clicks outside its content (the scrim). The router
            // only sees the main tree, so overlay hits are resolved here first.
            Event::Pointer(pointer) => self.resolve_pointer(pointer.position),
            Event::Scroll(ScrollEvent { position, .. }) => self.resolve_pointer(position),
            // Keyboard and IME both go to the focused widget: a composition is
            // meaningless without a target text field.
            Event::Key(_) => self.focus.focused().map(|target| Hit {
                target,
                native: None,
            }),
            Event::Ime(_) => self.focus.focused().map(|target| Hit {
                target,
                native: None,
            }),
        };
        let Some(hit) = target else {
            return false;
        };
        // A capturing participant takes the pointer itself: it is a real OS
        // view sitting above the framework's surface, so the framework has to
        // *subtract* rather than add. Routing the event on as well is how a
        // click on a map also activates the button behind it. Passthrough
        // participants never reach here — they are not marked native, so the
        // router resolves to the widget underneath and this is a no-op.
        if hit.native.is_some() {
            // Focus, unlike the pointer, is the framework's to give away: the
            // native view cannot take it without a call.
            if matches!(event, Event::Pointer(_)) {
                self.yield_native_focus(hit.target);
            }
            return true;
        }
        let route = self.router.route(hit.target);
        let mut handled = false;
        let mut focus_requests: Vec<ElementId> = Vec::new();
        let mut scroll_requests: Vec<(ElementId, ScrollOffset)> = Vec::new();
        let mut overlay_commands: Vec<OverlayCommand> = Vec::new();

        // Measured per dispatch rather than cached from the update pass: the
        // wheel may arrive before any update has run for this node, and a
        // container that clamped against a stale extent would stop at the wrong
        // end. Only the widgets that opted in are measured, and only along this
        // route — a wheel targets one subtree, and the rest of the tree has no
        // reason to be walked for it.
        let mut content_sizes: Vec<Option<Size>> = Vec::new();
        content_sizes.resize(self.nodes.len(), None);
        for stop in &route {
            let Some(index) = self.index_of(stop.id) else {
                continue;
            };
            if !self.nodes[index].dead && self.nodes[index].widget.needs_content_size() {
                content_sizes[index] = self.content_size(index);
            }
        }

        for stop in route {
            let Some(index) = self.index_of(stop.id) else {
                continue;
            };
            if self.nodes[index].dead {
                continue;
            }
            if self.dispatch_at(
                index,
                stop,
                event.clone(),
                &mut DispatchQueues {
                    focus_requests: &mut focus_requests,
                    scroll_requests: &mut scroll_requests,
                    overlay_commands: &mut overlay_commands,
                },
                &content_sizes,
            ) {
                handled = true;
                break;
            }
        }

        for id in focus_requests {
            self.set_focus(id);
        }
        for (id, offset) in scroll_requests {
            if let Some(index) = self.index_of(id)
                && !self.nodes[index].dead
            {
                self.nodes[index].scroll_offset = offset;
                self.layout_dirty = true;
            }
        }
        // Overlay work happens after the event, never inside it: opening one
        // mounts a subtree, which needs the whole host mutably while the widget
        // that asked for it is still borrowed out of that host.
        self.drain_overlay_commands(overlay_commands);
        handled
    }

    /// Applies overlay requests collected during a dispatch.
    fn drain_overlay_commands(&mut self, commands: Vec<OverlayCommand>) {
        // A command can open an overlay whose content immediately asks to close
        // it (or vice versa), so this loops until the queue stops growing rather
        // than draining once.
        let mut pending = commands;
        while !pending.is_empty() {
            for command in pending.drain(..) {
                match command {
                    OverlayCommand::Open {
                        handle,
                        owner,
                        kind,
                        modal,
                        bounds,
                        content,
                    } => {
                        self.open_overlay_content(
                            handle,
                            Some(owner),
                            kind,
                            modal,
                            bounds,
                            content,
                        );
                    }
                    OverlayCommand::Close(handle) => self.close_overlay(handle),
                }
            }
            pending = std::mem::take(&mut self.overlay_commands);
        }
    }

    fn dispatch_at(
        &mut self,
        index: usize,
        stop: RouteStop,
        event: Event,
        queues: &mut DispatchQueues<'_>,
        content_sizes: &[Option<Size>],
    ) -> bool {
        let node = &mut self.nodes[index];
        let mut cx = NodeEventContext {
            target: stop.id,
            bounds: node.bounds,
            event,
            content_size: content_sizes.get(index).copied().flatten(),
            focus_requests: &mut *queues.focus_requests,
            scroll_requests: &mut *queues.scroll_requests,
            ime_caret: &mut self.ime_caret,
            overlays: OverlayDispatch {
                commands: &mut *queues.overlay_commands,
                next_handle: &mut self.next_overlay_handle,
                owner: stop.id,
            },
        };
        node.widget.event(&mut cx, stop.id)
    }

    fn index_of(&self, id: ElementId) -> Option<usize> {
        self.nodes.iter().position(|node| node.id == id)
    }

    /// Takes the topmost overlay down when a press lands outside it and it is
    /// not modal. Returns whether the press was consumed by the dismissal.
    ///
    /// Modal overlays are excluded on purpose: they exist to swallow the window,
    /// and a press on the scrim is theirs to interpret — a dialog may
    /// legitimately insist on being answered. Non-modal overlays are the ones
    /// whose whole contract is "clicking away closes me", and the closure goes
    /// through [`WidgetHost::close_overlay`], so the opener is told through
    /// `overlay_closed` and can clear whatever state made it open in the first
    /// place.
    fn dismiss_top_overlay_on_press(&mut self, event: &Event) -> bool {
        let Event::Pointer(pointer) = event else {
            return false;
        };
        if !matches!(pointer.phase, PointerPhase::Down) {
            return false;
        }
        let Some(top) = self.overlay.top() else {
            return false;
        };
        let (id, modal, bounds) = (top.id, top.modal, top.bounds);
        if modal || bounds.contains(pointer.position) {
            return false;
        }
        self.close_overlay(id);
        true
    }

    /// The element under a point, overlays first — the pick a devtools panel
    /// makes when the user clicks the app to inspect it.
    ///
    /// The same resolution [`Self::resolve_pointer`] does, because a pick that
    /// answered differently from a click would select the wrong node: hovering
    /// an open dialog must pick the dialog's row, not the tree behind it.
    ///
    /// Returns `None` over empty space.
    pub fn element_at(&self, point: Point) -> Option<ElementId> {
        self.resolve_pointer(point).map(|hit| hit.target)
    }

    /// The node under a point, overlays first — [`Self::element_at`] with the
    /// tree position attached.
    ///
    /// A panel needs the node, not just the id: it walks to the parent for the
    /// breadcrumb and reads the bounds for the highlight box.
    pub fn node_at_point(&self, point: Point) -> Option<NodeId> {
        let id = self.element_at(point)?;
        self.index_of(id).map(NodeId)
    }

    /// Resolves a pointer target, letting an open overlay take priority over the
    /// main tree. A modal overlay returns its id even for points outside its
    /// content (the scrim), and a hit there falls through to the overlay root
    /// so the overlay can absorb or act on the dismissal.
    fn resolve_pointer(&self, point: Point) -> Option<Hit> {
        if let Some(overlay_id) = self.overlay.hit_test(point)
            && let Some(node) = self.overlay_node(overlay_id)
        {
            let target = self
                .hit_test_overlay_subtree(node, point)
                .unwrap_or(self.nodes[node].id);
            return Some(Hit {
                target,
                native: None,
            });
        }
        self.router.resolve_target(point)
    }

    /// The pointer shape for a point: whatever the widget under it asks for, or
    /// the platform default over empty space.
    ///
    /// Resolution goes through `resolve_pointer` rather than the hit tester
    /// directly, so an open overlay answers for its own subtree and the cursor
    /// follows the same element a click would. Overlay nodes live in the same
    /// `nodes` vec as everything else -- `overlay_node` hands back an index into
    /// it -- so `index_of` covers both trees.
    fn resolve_cursor(&self, point: Point) -> CursorStyle {
        let Some(hit) = self.resolve_pointer(point) else {
            return CursorStyle::Default;
        };
        self.index_of(hit.target)
            .map(|index| self.nodes[index].widget.cursor())
            .unwrap_or(CursorStyle::Default)
    }

    // --- native composition ------------------------------------------------

    /// Gives the host a provider for native participants.
    ///
    /// Without one every `NativeRequest` is refused. That is the honest answer:
    /// a host that cannot composite must not hand out participants that draw
    /// nothing, because a blank rectangle is indistinguishable from one that
    /// was never asked for.
    pub fn set_native_provider(&mut self, provider: impl NativeViewProvider + 'static) {
        self.native = Some(NativeViewController::boxed(provider));
    }

    /// Takes the provider away, tearing every participant down first.
    pub fn clear_native_provider(&mut self) {
        let nodes: Vec<usize> = self.participants.iter().map(|p| p.node).collect();
        for index in nodes {
            self.retire_participant(index);
        }
        self.native = None;
    }

    pub fn has_native_provider(&self) -> bool {
        self.native.is_some()
    }

    /// Live participants.
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    /// Slots currently bound.
    pub fn participant_slots(&self) -> Vec<NativeSlot> {
        self.participants.iter().map(|p| p.slot).collect()
    }

    /// The last state pushed down per participant.
    pub fn participant_states(&self) -> Vec<(NativeSlot, NativeState)> {
        self.participants
            .iter()
            .filter_map(|p| p.last.map(|state| (p.slot, state)))
            .collect()
    }

    /// Drains any snapshot pixels the native provider has queued.
    ///
    /// Called by the app runner before each frame, so a frozen participant's
    /// texture is imported exactly once — the frame its transition starts — and
    /// never lingers in the host. Empty when nothing was frozen (or no provider
    /// is installed).
    pub fn take_native_captures(&mut self) -> Vec<NativeCapture> {
        match &mut self.native {
            Some(controller) => controller.take_native_captures(),
            None => Vec::new(),
        }
    }

    /// Slots the provider was told to create and never told to destroy. Should
    /// be empty once the tree is torn down; anything left is a native view the
    /// framework can no longer name.
    pub fn unreleased_participants(&self) -> Vec<NativeSlot> {
        match &self.native {
            Some(controller) => controller.unreleased().collect(),
            None => Vec::new(),
        }
    }

    /// The participant holding OS focus, if any.
    pub fn native_focused(&self) -> Option<NativeSlot> {
        self.native_focus
    }

    /// Freezes a participant into a texture, standing in for its live view.
    ///
    /// The live view stays bound — it is *hidden*, not destroyed — because a
    /// transition's end hands it back and re-creating an OS view is the
    /// expensive thing. [`WidgetHost::thaw_native`] undoes this, and must be
    /// called exactly once per participant that got `Some`.
    ///
    /// `Ok(None)` means the provider cannot snapshot. That is not a failure:
    /// the participant carries on being composited live, and the caller gets to
    /// decide what a transition looks like without a frozen texture (the honest
    /// answer is usually "the native view simply does not animate"). It is
    /// deliberately distinct from `Err`, which means the caller did something
    /// illegal — freezing twice, or naming a slot that is not bound.
    pub fn freeze_native(
        &mut self,
        slot: NativeSlot,
    ) -> Result<Option<NativeSnapshot>, NativeError> {
        let Some(controller) = self.native.as_mut() else {
            return Err(NativeError::NotBound(slot));
        };
        // No "already frozen" check of our own: that invariant belongs to the
        // controller, which is the only thing that can answer it truthfully — a
        // second copy here would be a second answer that can drift.
        let Some(snapshot) = controller.snapshot(slot)? else {
            // Nothing was frozen, so there is nothing to record — and noting it
            // anyway would make a later `thaw` look legitimate.
            return Ok(None);
        };
        let Some(node) = self
            .participants
            .iter()
            .find(|p| p.slot == slot)
            .map(|p| p.node)
        else {
            // The provider handed back a texture, so ownership travelled; it has
            // to travel straight back or the resource is stranded.
            let _ = controller.restore(slot);
            return Err(NativeError::NotBound(slot));
        };
        self.frozen.push(Frozen {
            node,
            slot,
            snapshot,
            anim: None,
            composite: None,
        });
        Ok(Some(snapshot))
    }

    /// Freezes a participant and runs `anim` against the frozen texture,
    /// thawing it when the animation finishes.
    ///
    /// The point of the whole exercise: a transition is the one thing an OS
    /// view cannot do, and ending it is as much part of the contract as
    /// starting it — a participant left frozen is a texture standing in for a
    /// view that is hidden and will never come back.
    ///
    /// `Ok(None)` means the provider cannot snapshot, and in that case nothing
    /// is animated and nothing is frozen: fading a texture you cannot take is
    /// not a thing to attempt.
    pub fn begin_native_transition(
        &mut self,
        slot: NativeSlot,
        anim: AnimKind,
    ) -> Result<Option<NativeSnapshot>, NativeError> {
        let snapshot = self.freeze_native(slot)?;
        let Some(snapshot) = snapshot else {
            return Ok(None);
        };
        let id = self.native_anim.start(anim, self.anim_clock);
        if let Some(entry) = self.frozen.iter_mut().find(|f| f.slot == slot) {
            entry.anim = Some(id);
        }
        Ok(Some(snapshot))
    }

    /// Advances the native transitions and ends the ones that finished.
    fn tick_native_transitions(&mut self) -> bool {
        if self.native_anim.is_empty() {
            return false;
        }
        let now = self.anim_clock;
        let samples = self.native_anim.tick(now);
        let mut finished = Vec::new();
        for sample in samples {
            let Some(entry) = self.frozen.iter_mut().find(|f| f.anim == Some(sample.id)) else {
                continue;
            };
            if sample.finished {
                finished.push(entry.slot);
            } else {
                // A frozen native transition drives a single channel.
                entry.composite = sample.compositor.first().copied();
            }
        }
        // Thawing is what ends a transition, and it is the host doing it
        // rather than the caller: the caller is the thing that might have
        // scrolled away or unmounted by the time the tween parks.
        for slot in finished {
            let _ = self.thaw_native(slot);
        }
        !self.native_anim.is_empty()
    }

    /// Hands a frozen participant its live view back.
    ///
    /// Idempotence would be the wrong shape here: ownership moves exactly once,
    /// and a second `restore` either frees a texture nobody owns or leaks the
    /// one somebody already got. So thawing what is not frozen is an error.
    pub fn thaw_native(&mut self, slot: NativeSlot) -> Result<(), NativeError> {
        let Some(controller) = self.native.as_mut() else {
            return Err(NativeError::NotBound(slot));
        };
        // Ownership moves here or it does not move at all: the same reasoning
        // the controller applies to its own invariant set, and deliberately not
        // duplicated. To err after this point would leave the provider holding a
        // frozen view and the host believing it is live.
        controller.restore(slot)?;
        // The record has to go, or it outlives the participant it describes:
        // nothing else consults `frozen` for a slot once the slot is gone. The
        // *next frame* is what puts the live view back on screen, because
        // `sync_native` recomputes visibility from this list.
        if let Some(position) = self.frozen.iter().position(|f| f.slot == slot) {
            self.frozen.remove(position);
        }
        Ok(())
    }

    /// Slots currently showing a frozen texture instead of a live view.
    pub fn frozen_participants(&self) -> Vec<NativeSlot> {
        self.frozen.iter().map(|f| f.slot).collect()
    }

    /// The texture standing in for the participant bound to `node`.
    fn frozen_snapshot(&self, node: usize) -> Option<NativeSnapshot> {
        self.frozen
            .iter()
            .find(|f| f.node == node)
            .map(|f| f.snapshot)
    }

    /// This frame's tween for the frozen participant at `node`.
    ///
    /// Travels the same path an overlay's entry animation does — handed to
    /// `paint_node` and folded into every node it writes — because a frozen
    /// texture is drawn by the ordinary paint pass. That is the whole reason
    /// 16.4 needs no second animation mechanism.
    fn frozen_composite(&self, node: usize) -> SmallVec<[CompositorProp; 2]> {
        self.frozen
            .iter()
            .find(|f| f.node == node)
            .and_then(|f| f.composite)
            .into_iter()
            .collect()
    }

    /// Whether a participant is currently replaced by a frozen texture.
    fn is_frozen(&self, slot: NativeSlot) -> bool {
        self.frozen.iter().any(|f| f.slot == slot)
    }

    /// Device-pixel scale handed to participants.
    ///
    /// A change does not dirty layout — logical geometry is unchanged — but it
    /// does make every participant's stored state stale, so the next frame
    /// re-applies them.
    pub fn set_scale(&mut self, scale: f64) {
        if scale.is_finite() && scale > 0.0 {
            self.scale = scale;
        }
    }

    pub fn scale(&self) -> f64 {
        self.scale
    }

    /// Tears one participant down: destroy the view, release the slot back to
    /// the pool, and drop the node's claim to the hit region.
    fn retire_participant(&mut self, index: usize) {
        let Some(position) = self.participants.iter().position(|p| p.node == index) else {
            return;
        };
        let participant = self.participants.remove(position);
        // Drop the freeze record *before* the controller unwinds it, because the
        // slot is about to go back in the pool: a record that outlived its
        // participant would answer `is_frozen` for whatever the pool hands the
        // slot to next, and a recycled row would come back hidden with its
        // texture drawn nowhere — for no reason the caller can observe.
        if let Some(position) = self.frozen.iter().position(|f| f.slot == participant.slot) {
            self.frozen.remove(position);
        }
        if let Some(controller) = self.native.as_mut() {
            // A frozen participant is restored first by the controller, so the
            // snapshot texture cannot outlive the view that owned it.
            let _ = controller.destroy(participant.slot);
        }
        self.native_pool.release(&participant.key);
        if self.native_focus == Some(participant.slot) {
            self.native_focus = None;
            // The participant that held focus is gone. Hand it back, but only
            // to an element that is still mounted — a retiring row can take the
            // element it displaced with it.
            if let Some(previous) = self.native_focus_return.take() {
                let alive = self
                    .index_of(previous)
                    .is_some_and(|index| !self.nodes[index].dead);
                if alive && self.focus.focused().is_none() {
                    self.set_focus(previous);
                }
            }
        }
        self.pending_native_clears.push(index);
    }

    /// Pushes each participant's desired state down, but only when it changed.
    ///
    /// The diff lives here rather than in the controller because this is the
    /// only place that knows both the last state and the current frame's
    /// geometry — and because "nothing changed" is a statement about frames,
    /// not about individual calls.
    fn sync_native(&mut self, stats: &mut FrameStats) {
        // Nodes unmount by being marked dead, not by being removed, so this is
        // the only place that can notice a participant's node is gone.
        let dead: Vec<usize> = self
            .participants
            .iter()
            .filter(|p| self.nodes.get(p.node).is_none_or(|node| node.dead))
            .map(|p| p.node)
            .collect();
        for index in dead {
            self.retire_participant(index);
        }

        let clears = std::mem::take(&mut self.pending_native_clears);
        for index in clears {
            if let Some(node) = self.nodes.get_mut(index) {
                node.native = None;
                node.native_hidden = false;
            }
            self.refresh_hit_region(index);
        }

        if self.native.is_none() || self.participants.is_empty() {
            return;
        }

        let viewport = Rect::from_origin_size(Point::ZERO, self.size);
        // A modal overlay owns the screen. The participant stays bound and is
        // hidden instead of being destroyed: it will be straight back when the
        // modal closes, and recreating an OS view is the expensive part.
        let modal_up = self.overlay.top().is_some_and(|top| top.modal);
        let scale = self.scale;

        self.reconcile_clipping(viewport, stats);

        let mut updates: Vec<(NativeSlot, NativeState)> = Vec::new();
        for participant in &self.participants {
            let node = &self.nodes[participant.node];
            let bounds = node.bounds;
            let clip = self.clip(participant.node, bounds, viewport);
            let covered = clip.map_or(bounds, |clip| bounds.intersect(clip));
            let visible = !covered.is_empty()
                && !participant.clip_starved
                && !modal_up
                && !node.is_overlay
                && !self.is_in_overlay(participant.node)
                // A frozen participant has handed its pixels to the framework
                // for the length of a transition. Leaving the live view visible
                // would draw it *and* the texture — and since the view cannot
                // be transformed, the two would visibly separate the moment the
                // tween moved.
                && !self.is_frozen(participant.slot);
            let state = NativeState {
                // The placement, never the intersection: a provider that fitted
                // its content to a clipped rect would rescale instead of crop.
                rect: bounds,
                clip,
                scale,
                visible,
                // Paint order: a participant declared later sits above one
                // declared earlier, which is what `z` means to a provider.
                z: participant.node as u32,
                input: participant.input,
                mode: participant.mode,
            };
            if participant.last != Some(state) {
                updates.push((participant.slot, state));
            }
        }

        // Pushed in a scope of its own: recording what was applied needs `self`
        // again, and the controller is borrowed out of `self`.
        let mut applied: Vec<(NativeSlot, NativeState)> = Vec::new();
        if let Some(controller) = self.native.as_mut() {
            for (slot, state) in updates {
                if controller.apply(slot, state).is_ok() {
                    applied.push((slot, state));
                }
            }
        }

        for (slot, state) in applied {
            let index = self.participants.iter().position(|p| p.slot == slot);
            if let Some(position) = index {
                self.participants[position].last = Some(state);
                let node = self.participants[position].node;
                // Only a capturing participant claims the hit region, and only
                // while it is actually on screen. A passthrough one is
                // composited in the same place but does not take the pointer, so
                // the widget underneath must keep receiving events; and an
                // off-screen participant must be out of reach entirely, or a
                // click would land on a native surface that is not there.
                self.nodes[node].native = match self.participants[position].input {
                    InputMode::Capture if state.visible => Some(slot.id),
                    _ => None,
                };
                self.nodes[node].native_hidden = !state.visible;
                self.refresh_hit_region(node);
            }
            stats.native_applied += 1;
        }
    }

    /// Moves participants whose provider cannot crop out of the way of a clip
    /// that would be honoured by rescaling, or hides them.
    ///
    /// This is the whole answer to "what does a host do when the platform
    /// cannot crop": the geometry the host computes is right, the provider is
    /// honest about what it can do, and the *mode* is what gives.
    ///
    /// - A provider that crops — itself ([`ClipSupport::Provider`]) or via its
    ///   platform ([`ClipSupport::Platform`]) — needs none of this; it is sent
    ///   the placement and the crop and does the right thing.
    /// - A provider that cannot crop gets the participant re-negotiated into a
    ///   mode whose pixels the *framework* owns, where cropping is free. That
    ///   costs one destroy/create pair, once.
    /// - If it will accept no such mode, the participant is hidden. Drawing it
    ///   unclipped (device case D6) and rescaling it are both wrong and both
    ///   silent; a gap is wrong and *visible*, and `native_clip_hidden` says so.
    ///
    /// The degradation is one-way for the lifetime of the participant. Putting
    /// it back the moment the clip stops biting would destroy and re-create a
    /// native view on every scrolled frame at the boundary.
    fn reconcile_clipping(&mut self, viewport: Rect, stats: &mut FrameStats) {
        let Some(caps) = self.native.as_ref().map(|c| c.capabilities()) else {
            return;
        };
        if caps.can_crop() {
            return;
        }
        for position in 0..self.participants.len() {
            let participant = self.participants[position];
            let bounds = self.nodes[participant.node].bounds;
            let clip = self.clip(participant.node, bounds, viewport);
            let needs_crop = clip.is_some_and(|clip| !clip.contains_rect(bounds));
            if !needs_crop {
                // Whole again: whatever was decided while it was clipped no
                // longer applies, and the next clip gets a fresh decision.
                self.participants[position].clip_starved = false;
                continue;
            }
            if participant.mode.is_host_composited() || participant.clip_starved {
                continue;
            }
            match caps.negotiate_croppable(participant.modes) {
                Some(mode) => {
                    let rebound = self.native.as_mut().and_then(|controller| {
                        controller
                            .rebind(participant.slot, participant.kind, &[mode])
                            .ok()
                    });
                    match rebound {
                        Some(mode) => {
                            // The new view has never been told anything, so the
                            // next state is pushed unconditionally.
                            self.participants[position].mode = mode;
                            self.participants[position].last = None;
                            stats.native_degraded += 1;
                        }
                        None => {
                            self.participants[position].clip_starved = true;
                            stats.native_clip_hidden += 1;
                        }
                    }
                }
                None => {
                    self.participants[position].clip_starved = true;
                    stats.native_clip_hidden += 1;
                }
            }
        }
    }

    fn refresh_hit_region(&mut self, index: usize) {
        let Some(node) = self.nodes.get(index) else {
            return;
        };
        if node.dead {
            return;
        }
        if node.native_hidden {
            // Off screen: out of reach entirely, not merely stripped of its
            // native claim. A deep node's region outranks a shallow one's, so
            // leaving it in place would keep eating clicks meant for whatever is
            // actually painted there.
            self.router.hits.remove(node.id);
            return;
        }
        let z = node.depth as i32 * 1_000_000 + index as i32;
        self.router.hits.set_region(
            node.id,
            HitRegion {
                bounds: node.bounds,
                native: node.native,
                z,
            },
        );
    }

    /// The crop `rect` is under, or `None` when nothing clips it.
    ///
    /// Ancestors stand in for clips. The widget layer has no overflow/clip API
    /// yet, and a native view does not clip itself to anything — so without
    /// this a participant in a scrolled container floats over content it should
    /// be behind (device case D6). When a real clip API lands, this narrows to
    /// the ancestors that actually clip.
    ///
    /// `None` rather than a rect that happens to contain `rect`, because "no
    /// one clips me" and "clipped to nothing" are different statements and only
    /// the second one is a problem for a provider.
    fn clip(&self, index: usize, rect: Rect, viewport: Rect) -> Option<Rect> {
        let mut clipped = rect.intersect(viewport);
        let mut parent = self.nodes[index].parent;
        while let Some(parent_index) = parent {
            clipped = clipped.intersect(self.nodes[parent_index].bounds);
            parent = self.nodes[parent_index].parent;
        }
        if clipped.contains_rect(rect) {
            None
        } else {
            Some(clipped)
        }
    }

    /// Hands OS focus to `id`'s participant, or takes it back.
    ///
    /// Exclusive by construction: two owners of IME is a visible bug — the
    /// candidate window jumps between the framework's caret and the native
    /// field — so the framework yields rather than sharing (decision D4).
    fn yield_native_focus(&mut self, id: ElementId) {
        let slot = self
            .index_of(id)
            .and_then(|index| self.participants.iter().find(|p| p.node == index))
            .map(|p| p.slot);
        match (slot, self.native_focus) {
            (Some(slot), current) if current == Some(slot) => {}
            (Some(slot), _) => {
                let Some(controller) = self.native.as_mut() else {
                    return;
                };
                if controller.focus(slot).is_err() {
                    // The provider cannot focus (or the slot is gone). Leaving
                    // the framework focused is the right failure: a focus that
                    // went nowhere at all is worse than one that did not move.
                    return;
                }
                // Yielding means the framework lets go, not that it keeps a
                // caret in reserve: `ime_caret_rect` is masked while native
                // focus is held, and keys have no framework target at all.
                if let Some(previous) = self.focus.focused() {
                    self.clear_focus_flag(previous);
                    self.native_focus_return = Some(previous);
                }
                self.focus.blur();
                self.native_focus = Some(slot);
            }
            (None, Some(current)) => {
                if let Some(controller) = self.native.as_mut() {
                    let _ = controller.blur(current);
                }
                self.native_focus = None;
                // Hand focus back, unless the caller has already named a new
                // owner — in which case `set_focus` has just run and its choice
                // wins over the element the participant displaced.
                match self.native_focus_return.take() {
                    Some(previous) if self.focus.focused().is_none() => self.set_focus(previous),
                    _ => {}
                }
            }
            (None, None) => {}
        }
    }

    // --- focus -------------------------------------------------------------

    pub fn focused(&self) -> Option<ElementId> {
        self.focus.focused()
    }

    /// Whether focus was last moved by keyboard navigation (Tab / Shift+Tab).
    ///
    /// Widgets read this through their own interaction state to decide whether
    /// to paint a focus ring: keyboard users get a visible ring, mouse users do
    /// not, which matches platform convention.
    pub fn keyboard_navigation(&self) -> bool {
        self.keyboard_navigation
    }

    /// Composition caret rectangle the focused text widget last reported, or
    /// `None` when nothing is composing. The runner reads this after an IME
    /// event to anchor the OS input-method popup.
    ///
    /// `None` while a native participant holds focus: it owns the candidate
    /// window now, and publishing a framework caret as well is what makes the
    /// candidate window jump between two owners.
    pub fn ime_caret_rect(&self) -> Option<Rect> {
        if self.native_focus.is_some() {
            return None;
        }
        self.ime_caret
    }

    pub fn set_focus(&mut self, id: ElementId) {
        // Pointer-driven focus: a keyboard-initiated ring should disappear once
        // the mouse takes over (focus follows the pointer).
        self.keyboard_navigation = false;
        if let Some(previous) = self.focus.focused() {
            self.clear_focus_flag(previous);
        }
        self.focus.focus(id);
        self.set_focus_flag(id, true);
        self.yield_native_focus(id);
    }

    pub fn focus_next(&mut self) {
        self.keyboard_navigation = true;
        self.move_focus(FocusDirection::Forward);
    }

    pub fn focus_prev(&mut self) {
        self.keyboard_navigation = true;
        self.move_focus(FocusDirection::Backward);
    }

    fn move_focus(&mut self, direction: FocusDirection) {
        let previous = self.focus.focused();
        let next = self.focus.traverse(direction);
        if previous != next {
            if let Some(previous) = previous {
                self.set_focus_flag(previous, false);
            }
            if let Some(next) = next {
                self.set_focus_flag(next, true);
            }
        }
    }

    fn set_focus_flag(&mut self, id: ElementId, focused: bool) {
        if let Some(index) = self.index_of(id) {
            if self.nodes[index].dead {
                return;
            }
            let node = &mut self.nodes[index];
            set_widget_focused(node.widget.as_mut(), focused);
        }
    }

    fn clear_focus_flag(&mut self, id: ElementId) {
        self.set_focus_flag(id, false);
    }

    // --- output ------------------------------------------------------------

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// The resource registry backing the scene: glyph runs, images, blobs.
    ///
    /// Presenters receive both the scene and this table, so renderer backends
    /// can resolve the payloads scene nodes reference by id.
    pub fn resources(&self) -> &ResourceRegistry {
        &self.resources
    }

    /// Registers decoded image pixels and returns the resource id a
    /// [`floom_widgets::Image`] widget renders from.
    ///
    /// `pixels` must be RGBA8 **premultiplied**, row-major, `width * height * 4`
    /// bytes — the format the GPU path uploads verbatim and the software path
    /// documents. Returns `None` (without touching the registry) when the buffer
    /// does not match the declared extent, so a producer bug fails loudly at
    /// registration instead of corrupting a GPU upload later.
    pub fn register_image(
        &mut self,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> Option<ResourceId> {
        if pixels.len() != width as usize * height as usize * 4 {
            return None;
        }
        Some(self.resources.insert(Resource {
            kind: ResourceKind::Image {
                width,
                height,
                pixels: std::sync::Arc::from(pixels.into_boxed_slice()),
            },
        }))
    }

    /// Decodes encoded image bytes (PNG, JPEG, …) and registers the pixels.
    ///
    /// A convenience over [`decode_image`](image_codec::decode_image) +
    /// [`Self::register_image`]: the decoder owns the straight→premultiplied
    /// alpha conversion, so callers pass raw file bytes and get a resource id.
    /// Returns `None` when the bytes do not decode.
    pub fn register_image_decoded(&mut self, bytes: &[u8]) -> Option<ResourceId> {
        let decoded = image_codec::decode_image(bytes)?;
        self.register_image(decoded.width, decoded.height, decoded.pixels)
    }

    pub fn semantics(&self) -> &SemanticsTree {
        &self.semantics
    }

    pub fn last_stats(&self) -> FrameStats {
        self.last_stats
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}

/// Best-effort focus notification for widgets that model focus themselves.
///
/// A plain trait call: `Widget::set_focused` defaults to a no-op, so a widget
/// that paints no focus ring ignores it and one that does overrides it.
///
/// This used to downcast through a hand-written list of the engine's controls,
/// which meant the host had to be edited every time a focusable widget was
/// added — and a widget that lived above `floom-widgets` could not be listed at
/// all. The failure was silent: such a widget mounted, tracked focus, painted
/// no ring, and every test still passed. The same shape as
/// `overlay_opened` / `overlay_closed`: the host asks, the widget answers.
fn set_widget_focused(widget: &mut dyn WidgetAny, focused: bool) {
    widget.set_focused(focused);
}

// --- context implementations -----------------------------------------------

struct HostCreateContext<'a> {
    allocator: &'a mut IdAllocator,
}

impl CreateContext for HostCreateContext<'_> {
    fn alloc_child(&mut self) -> ElementId {
        self.allocator.alloc()
    }
}

/// Where a widget's overlay request goes when it is made from `update` rather
/// than from an event.
///
/// Same shape as [`OverlayDispatch`] — the event-time twin — but the owner is
/// the node being updated rather than a route stop, and the sink is optional so
/// the `overlay_opened`/`overlay_closed` callbacks cannot re-enter the drain
/// that is currently running them.
struct UpdateOverlaySink<'a> {
    commands: &'a mut Vec<OverlayCommand>,
    next_handle: &'a mut u64,
    owner: ElementId,
}

struct HostUpdateContext<'a> {
    dirty: bool,
    children_dirty: bool,
    /// A widget asked for the layout pass to run again — see
    /// [`UpdateContext::mark_layout_dirty`]. Folded into the host's own
    /// `layout_dirty` after the update pass, because the flag is a field of the
    /// host and this borrows the node out of it for the duration of `update`.
    layout_dirty: bool,
    /// The updating node's box (from the previous frame — `update` runs before
    /// layout), so a composite can anchor an overlay to itself.
    bounds: Rect,
    /// The host viewport, for a composite whose overlay spans the window.
    viewport: Rect,
    /// This node's content extent, measured by the host before the update pass
    /// borrowed anything — see [`UpdateContext::content_size`].
    content_size: Option<Size>,
    /// The frozen texture standing in for this node's participant, if its
    /// transition has taken the live view away.
    snapshot: Option<NativeSnapshot>,
    overlays: Option<UpdateOverlaySink<'a>>,
    /// The native door, present when the host has a provider.
    native: Option<NativeGrantSink<'a>>,
}

/// The host-side half of [`UpdateContext::request_native`].
///
/// Every field it touches is disjoint from the node being updated, which is why
/// a widget can be granted a participant in the middle of its own `update()`
/// without the host having to queue the request and answer later. The
/// alternative — queueing — would mean a widget could not know, within the
/// frame it asked, whether it got a native view or has to draw a fallback.
struct NativeGrantSink<'a> {
    node: usize,
    controller: &'a mut NativeViewController<Box<dyn NativeViewProvider>>,
    participants: &'a mut Vec<Participant>,
    pool: &'a mut NativeViewPool<SlotKey>,
    next_key: &'a mut u64,
    /// Nodes whose claim on the hit region must be dropped, deferred because
    /// the tree is borrowed while `update` runs.
    clears: &'a mut Vec<usize>,
}

impl NativeGrantSink<'_> {
    fn grant(&mut self, request: &NativeRequest) -> NativeGrant {
        if let Some(position) = self.participants.iter().position(|p| p.node == self.node) {
            let existing = self.participants[position];
            // Re-stating an unchanged request is the common case — `update`
            // runs every frame — and must be free.
            if existing.kind == request.kind
                && existing.requested_key == request.key
                && existing.input == request.input
            {
                return NativeGrant::Granted(existing.mode);
            }
            // A different key means a different row; a different kind means a
            // different view. Either way the old one has to be destroyed
            // before the new one is bound, or the provider is left holding two
            // handles for one slot.
            let _ = self.controller.destroy(existing.slot);
            self.pool.release(&existing.key);
            self.participants.remove(position);
        }

        let requested_key = request.key;
        // Identity-less participants get a minted key so two of them can never
        // share a slot. Rows that pass a real key share on purpose: that is
        // what lets scrolling recycle a handful of views.
        let key = if requested_key != 0 {
            SlotKey::Caller(requested_key)
        } else {
            let minted = *self.next_key;
            *self.next_key += 1;
            SlotKey::Minted(minted)
        };
        let slot = self.pool.acquire(key);
        match self.controller.bind(slot, request.kind, request.modes) {
            Ok(mode) => {
                self.participants.push(Participant {
                    node: self.node,
                    slot,
                    requested_key,
                    key,
                    kind: request.kind,
                    modes: request.modes,
                    mode,
                    input: request.input,
                    last: None,
                    clip_starved: false,
                });
                NativeGrant::Granted(mode)
            }
            // No acceptable mode: take the slot back rather than letting it
            // leak into the free list's accounting.
            Err(_) => {
                self.pool.release(&key);
                self.clears.push(self.node);
                NativeGrant::Refused
            }
        }
    }
}

impl HostUpdateContext<'_> {
    /// A context for the overlay callbacks: geometry only, no overlay door.
    fn callback(bounds: Rect, viewport: Rect) -> Self {
        Self {
            dirty: false,
            children_dirty: false,
            layout_dirty: false,
            bounds,
            viewport,
            content_size: None,
            snapshot: None,
            overlays: None,
            native: None,
        }
    }
}

impl UpdateContext for HostUpdateContext<'_> {
    fn mark_dirty(&mut self, _id: ElementId) {
        self.dirty = true;
    }

    fn mark_children_dirty(&mut self, _id: ElementId) {
        self.children_dirty = true;
    }

    fn mark_layout_dirty(&mut self, _id: ElementId) {
        self.layout_dirty = true;
    }

    /// The extent of this node's descendants, from the most recent layout.
    ///
    /// Read during `update`, which runs *before* layout, so this is the
    /// previous frame's geometry. That is the latest figure that exists: there
    /// is no layout to read yet. For a scroll container it is the right one —
    /// the content it is clamping against is the content it last laid out, and
    /// a frame of lag on a size that has not been re-measured yet is invisible.
    fn content_size(&mut self, _id: ElementId) -> Option<Size> {
        self.content_size
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn viewport(&self) -> Rect {
        self.viewport
    }

    fn open_overlay(&mut self, request: OverlayRequest) -> bool {
        let Some(sink) = self.overlays.as_mut() else {
            // No door here: the caller is a lifecycle callback, and letting it
            // open an overlay would re-enter the drain that is invoking it.
            return false;
        };
        // Mint and build now, exactly as the event path does, so the queued
        // request is complete and the content already holds its own handle.
        let handle = OverlayHandle::new(*sink.next_handle);
        *sink.next_handle += 1;
        let owner = sink.owner;
        let content = (request.content)(handle);
        sink.commands.push(OverlayCommand::Open {
            handle,
            owner,
            kind: request.kind,
            modal: request.modal,
            bounds: request.bounds,
            content,
        });
        true
    }

    fn close_overlay(&mut self, handle: OverlayHandle) -> bool {
        let Some(sink) = self.overlays.as_mut() else {
            return false;
        };
        sink.commands.push(OverlayCommand::Close(handle));
        true
    }

    fn request_native(&mut self, request: &NativeRequest) -> NativeGrant {
        // Answered inline rather than queued, unlike the overlay door. An
        // overlay mounts a subtree and therefore needs the host to itself; a
        // participant touches only fields disjoint from the node being updated,
        // so a widget can find out — within the frame it asked — whether it got
        // a native view or has to draw its own fallback.
        match self.native.as_mut() {
            Some(sink) => sink.grant(request),
            None => NativeGrant::Refused,
        }
    }

    fn native_snapshot(&self) -> Option<NativeSnapshot> {
        self.snapshot
    }
}

/// An interned glyph-run entry: the resource id plus the identity it was
/// interned under, so a hash lookup can verify instead of trust.
#[derive(Debug, Clone)]
struct GlyphCacheEntry {
    text: String,
    size_bits: u32,
    /// The CSS number, because that is what the hash carries; keeping the enum
    /// here as well would be a second representation that can drift from it.
    weight: u16,
    id: ResourceId,
}

/// The interned text nodes for one styled description.
///
/// The description is kept alongside the ids so a hash collision degrades to a
/// re-intern rather than to the wrong resource — the same rule the plain
/// glyph cache follows.
#[derive(Debug, Clone)]
struct StyledGlyphCacheEntry {
    styled: StyledText,
    /// One entry per ink in the description: the scene carries one colour per
    /// text node, so a line with three inks is three nodes over one layout.
    nodes: Vec<(floom_types::Color, ResourceId)>,
    /// The size of the shaped text, shared by every node of the description.
    size: floom_types::Size,
}

/// How many styled descriptions the host keeps shaped at once.
///
/// The bound is the point: an editor's description changes on every
/// keystroke, so an unbounded cache would grow for as long as the editor is
/// open, and the entries it holds are a cache of a shape that is cheap to
/// rebuild.
const STYLED_GLYPH_CACHE_LIMIT: usize = 256;

/// Hashes a styled description for [`WidgetHost::styled_glyph_cache`].
///
/// Everything that can change the shaped result belongs in the key: the text,
/// the base style, the typeface, the wrapping, the tab width, the break
/// width, and every span. Leaving the spans out would be the same bug the
/// plain cache had before the weight joined its key — a highlighted line
/// handed back the runs of an unhighlighted one, which is the same text in
/// the wrong ink with nothing downstream to tell.
fn styled_glyph_hash(styled: &StyledText) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    styled.text.hash(&mut hasher);
    styled.base.size.to_bits().hash(&mut hasher);
    styled.base.weight.to_value().hash(&mut hasher);
    styled.base.color.hash(&mut hasher);
    styled.family.hash(&mut hasher);
    styled.wrap.hash(&mut hasher);
    styled.tab_width.hash(&mut hasher);
    styled.max_width.map(f32::to_bits).hash(&mut hasher);
    styled.spans.len().hash(&mut hasher);
    for span in &styled.spans {
        span.range.start.hash(&mut hasher);
        span.range.end.hash(&mut hasher);
        span.style.size.to_bits().hash(&mut hasher);
        span.style.weight.to_value().hash(&mut hasher);
        span.style.color.hash(&mut hasher);
    }
    hasher.finish()
}

/// Hashes (text, font size, weight) for [`WidgetHost::glyph_cache`].
///
/// Text content and size together decide glyph ids and positions, so both
/// belong in the key — and so does the weight, because two weights of one
/// string are two *different* sets of glyph ids. Leaving it out would not
/// merely cost a re-shape: a `Bold` label would be handed the `Normal` runs
/// for its own text, which is the same string at the same size in the wrong
/// face, and the only symptom is that every weighted label in a design
/// system renders flat.
fn glyph_hash(text: &str, font_size: f32, weight: FontWeight) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    font_size.to_bits().hash(&mut hasher);
    weight.to_value().hash(&mut hasher);
    hasher.finish()
}

struct NodePaintContext<'a> {
    scene: &'a mut Scene,
    resources: &'a mut ResourceRegistry,
    glyphs: &'a mut HashMap<u64, GlyphCacheEntry>,
    styled_glyphs: &'a mut HashMap<u64, StyledGlyphCacheEntry>,
    shaper: &'a mut TextShaper,
    layout_cache: &'a mut LayoutCache,
    /// The overlay entry animation this node is being painted under, if any —
    /// or the node's own animation, which takes precedence. Applied to every
    /// scene node the context writes, so a whole overlay subtree fades or
    /// slides as one. Each channel (opacity folds alpha, a translation offsets
    /// the box) is applied to every node; scale and rotate are never folded
    /// here, only bracketed by the group scope.
    composite: SmallVec<[CompositorProp; 2]>,
    /// This frame's value from the animation running on this element, for the
    /// element to draw its own motion with.
    animation: Option<f64>,
    /// The width this node's text was wrapped against when it was measured.
    /// See [`TextMeasure::wrap_width`].
    wrap_width: f64,
    /// Last frame's scene node ids for this widget — and, in place, this
    /// frame's. Painting reuses them positionally (`upsert` on a match, no
    /// damage when the payload is unchanged; `insert` once the previous nodes
    /// run out), so the vector only ever grows at the tail: a frame that
    /// repaints the tree allocates nothing per node to do it.
    prev: Vec<SceneNodeId>,
    used: usize,
    bounds: Rect,
}

impl NodePaintContext<'_> {
    /// Consumes the context: the (in-place updated) id list minus the ids
    /// this frame no longer produced. Stale ids are retired here — removal
    /// needs the scene borrow the context already holds, and draining them
    /// straight out of the vector avoids a second allocation per node.
    fn finish(mut self) -> Vec<SceneNodeId> {
        for id in self.prev.drain(self.used..) {
            self.scene.remove(id);
        }
        self.prev
    }

    /// The interned id for (text, size) if the cache already holds it —
    /// allocation-free on the hit path (no key String, no run clones). This
    /// must run *before* shaping: interning after shaping still paid for the
    /// shaping and a full `Vec<GlyphRun>` clone on every frame the text did
    /// not change.
    fn cached_glyph_id(
        &self,
        text: &str,
        font_size: f32,
        weight: FontWeight,
    ) -> Option<ResourceId> {
        let entry = self.glyphs.get(&glyph_hash(text, font_size, weight))?;
        (entry.text == text
            && entry.size_bits == font_size.to_bits()
            && entry.weight == weight.to_value())
        .then_some(entry.id)
    }

    /// The interned text nodes for a styled description, one per ink.
    ///
    /// The scene carries one colour per text node, so a line with three inks
    /// is three nodes over one layout. Grouping the runs by their ink changes
    /// the *draw order* of the inks — it becomes the order they were first
    /// seen rather than the order they appear in the text — which for text
    /// that does not overlap is the same picture.
    fn styled_glyph_nodes(
        &mut self,
        styled: &StyledText,
    ) -> (Vec<(floom_types::Color, ResourceId)>, floom_types::Size) {
        let key = styled_glyph_hash(styled);
        if let Some(entry) = self.styled_glyphs.get(&key)
            && entry.styled == *styled
        {
            return (entry.nodes.clone(), entry.size);
        }
        // Shape the description exactly as the caller laid it out: the same
        // description, shaped by the same shaper, so the glyphs the caller
        // measured the caret against are the glyphs that get painted.
        let layout = floom_text::TextLayout::new(styled);
        let size = layout.size();
        let mut groups: Vec<(floom_types::Color, Vec<floom_text::GlyphRun>)> = Vec::new();
        for run in layout.runs() {
            let glyph_run = floom_text::GlyphRun {
                font: run.font.clone(),
                font_size: run.font_size,
                origin: run.origin,
                glyphs: run.glyphs.clone(),
            };
            match groups.iter_mut().find(|(color, _)| *color == run.color) {
                Some((_, runs)) => runs.push(glyph_run),
                None => groups.push((run.color, vec![glyph_run])),
            }
        }
        let mut nodes = Vec::with_capacity(groups.len());
        for (color, runs) in groups {
            // The string the runs were shaped from travels with the resource.
            // It is the only cheap chance to do so: shaping is lossy (a
            // ligature or a fallback font makes glyph id -> character
            // many-to-many), so a backend that needs characters rather than
            // ink cannot recover them later.
            let id = self.resources.insert(Resource {
                kind: ResourceKind::glyph_runs_with_text(runs, styled.text.as_str()),
            });
            nodes.push((color, id));
        }
        // The cache is bounded, because an editor produces a *new*
        // description on every keystroke and an unbounded map would grow for
        // as long as the editor is open. Dropping the whole map is the crude
        // way to bound it, and it is the right one here: the entries are a
        // cache of a shape that is cheap to rebuild, and a map that evicts one
        // entry at a time would need an order the hash map does not have.
        if self.styled_glyphs.len() >= STYLED_GLYPH_CACHE_LIMIT {
            self.styled_glyphs.clear();
        }
        self.styled_glyphs.insert(
            key,
            StyledGlyphCacheEntry {
                styled: styled.clone(),
                nodes: nodes.clone(),
                size,
            },
        );
        (nodes, size)
    }

    /// Interns a shaped glyph run, reusing the resource across frames.
    ///
    /// The key is the (text, font size, weight) hash — the same string at two
    /// sizes has different glyph ids and positions, and at two weights it has
    /// different glyph ids outright, so both must be part of it.
    /// Call [`Self::cached_glyph_id`] first; only a miss needs the runs.
    fn intern_glyphs(
        &mut self,
        text: &str,
        font_size: f32,
        weight: FontWeight,
        runs: Vec<floom_text::GlyphRun>,
    ) -> ResourceId {
        if let Some(id) = self.cached_glyph_id(text, font_size, weight) {
            return id;
        }
        // The string the runs were shaped from travels with the resource. It is
        // the only cheap chance to do so: shaping is lossy (a ligature or a
        // fallback font makes glyph id -> character many-to-many), so a backend
        // that needs characters rather than ink cannot recover them later.
        let id = self.resources.insert(Resource {
            kind: ResourceKind::glyph_runs_with_text(runs, text),
        });
        self.glyphs.insert(
            glyph_hash(text, font_size, weight),
            GlyphCacheEntry {
                text: text.to_string(),
                size_bits: font_size.to_bits(),
                weight: weight.to_value(),
                id,
            },
        );
        id
    }
}

impl PaintContext for NodePaintContext<'_> {
    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn animation_value(&self) -> Option<f64> {
        self.animation
    }

    fn fill_rect(&mut self, rect: Rect, color: floom_types::Color) {
        self.push(floom_scene::SceneNode::Rect {
            bounds: rect,
            fill: color.into(),
            radius: 0.0,
            shadow: None,
        });
    }

    fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: floom_types::Color) {
        self.push(floom_scene::SceneNode::Rect {
            bounds: rect,
            fill: color.into(),
            radius: radius.max(0.0),
            shadow: None,
        });
    }

    fn fill_rect_brush(&mut self, rect: Rect, brush: &floom_types::Brush) {
        self.push(floom_scene::SceneNode::Rect {
            bounds: rect,
            fill: brush.clone(),
            radius: 0.0,
            shadow: None,
        });
    }

    fn fill_rounded_rect_brush(&mut self, rect: Rect, radius: f32, brush: &floom_types::Brush) {
        self.push(floom_scene::SceneNode::Rect {
            bounds: rect,
            fill: brush.clone(),
            radius: radius.max(0.0),
            shadow: None,
        });
    }

    fn fill_path_brush(&mut self, path: &floom_types::Path, brush: &floom_types::Brush) {
        self.push(floom_scene::SceneNode::Path {
            path: std::sync::Arc::new(path.clone()),
            fill: Some(brush.clone()),
            stroke: None,
        });
    }

    /// A shadow is a *rect-shaped* request, so it rides on a rect node whose
    /// own fill contributes no ink: the silhouette is the rect, the blur and
    /// offset come from the shadow, and whatever the widget draws next lands
    /// on top. One node rather than a pending state latched onto the following
    /// fill — otherwise "the shadow this shape casts" and "this shape" would
    /// be two things the scene has to keep in step.
    fn draw_shadow(&mut self, rect: Rect, radius: f32, shadow: &floom_types::Shadow) {
        if shadow.color.a == 0 {
            return;
        }
        self.push(floom_scene::SceneNode::Rect {
            bounds: rect,
            fill: floom_types::Brush::Solid(floom_types::Color::TRANSPARENT),
            radius: radius.max(0.0),
            shadow: Some(*shadow),
        });
    }

    fn backdrop_filter(&mut self, rect: Rect, filter: &floom_types::Filter) {
        // An identity filter is "no filter": writing a node for it would make
        // every backend pay for a layer to do nothing.
        if filter.is_identity() {
            return;
        }
        self.push(floom_scene::SceneNode::BackdropFilter {
            rect,
            filter: *filter,
        });
    }

    /// A custom shader is the element's ink, so the box is the program's whole
    /// output — no rect is filled behind it, and an empty box is dropped
    /// instead of asking every backend for an offscreen pass over zero pixels.
    /// The skip counters live in the backends: the software renderer counts
    /// `shaders_skipped` and vello counts `shaders_drawn`, so "the shader ran"
    /// and "the shader was degraded" are both observable.
    fn draw_shader(&mut self, shader: &floom_types::Shader) {
        if self.bounds.width() <= 0.0 || self.bounds.height() <= 0.0 {
            return;
        }
        self.push(floom_scene::SceneNode::shader(self.bounds, shader.clone()));
    }

    fn fill_text(
        &mut self,
        origin: Point,
        text: &str,
        size: f32,
        weight: FontWeight,
        color: floom_types::Color,
    ) {
        if text.is_empty() || size <= 0.0 {
            return;
        }
        // Shape within the laid-out box so wrapped text paints as wrapped: the
        // measure callback sized this node against the same width.
        //
        // The measured width wins where it is *wider* than the box, and the reason
        // is a rounding, not a preference. A measure leaf's box comes back from
        // the layout engine rounded to whole pixels, so a string whose natural
        // width lands in the upper half-pixel of that box would wrap by one word
        // against it — a label that measured in one line, laid out in one line,
        // and then painted in two. The measure closure's available width is what
        // the text was actually wrapped against, so preferring it makes the two
        // agree exactly. A string that really is meant to wrap still does: its
        // available width is the box.
        let box_width = if self.bounds.width() > 0.0 {
            self.bounds.width()
        } else {
            f64::INFINITY
        };
        let max_width = if self.wrap_width > box_width {
            self.wrap_width
        } else {
            box_width
        };
        let options = TextLayoutOptions {
            max_width,
            font_size: size,
            align: TextAlign::Start,
            weight,
        };
        // The intern cache is checked *before* any shaping: a repeat frame
        // must not re-shape, clone the shaped runs, or mint a key String for
        // a text that has not changed. Shaping is the expensive part (parse /
        // script segmentation / glyph placement); the cache hit keeps glyph
        // runs — and their font data — stable across frames.
        let glyphs = match self.cached_glyph_id(text, size, weight) {
            Some(id) => id,
            None => {
                let shaper = &mut *self.shaper;
                let laid = self
                    .layout_cache
                    .get_or_insert_with(text, &options, || shaper.shape(text, &options));
                let runs = laid.runs.clone();
                self.intern_glyphs(text, size, weight, runs)
            }
        };
        // The laid-out extent is a cheap cache lookup either way (no clones,
        // no re-shaping on a hit).
        let shaper = &mut *self.shaper;
        let laid_size = self
            .layout_cache
            .get_or_insert_with(text, &options, || shaper.shape(text, &options))
            .size;
        self.push(floom_scene::SceneNode::Text {
            origin,
            size: laid_size,
            glyphs,
            color,
        });
    }

    fn fill_styled_text(&mut self, origin: Point, styled: &StyledText) {
        if styled.text.is_empty() || styled.base.size <= 0.0 {
            return;
        }
        let (nodes, size) = self.styled_glyph_nodes(styled);
        for (color, glyphs) in nodes {
            self.push(floom_scene::SceneNode::Text {
                origin,
                size,
                glyphs,
                color,
            });
        }
    }

    fn draw_image(&mut self, bounds: Rect, image: ResourceId) {
        self.push(floom_scene::SceneNode::image(bounds, image));
    }

    fn fill_path(&mut self, path: &floom_types::Path, color: floom_types::Color) {
        self.push(floom_scene::SceneNode::filled_path(path.clone(), color));
    }

    fn stroke_path(&mut self, path: &floom_types::Path, stroke: &floom_types::Stroke) {
        self.push(floom_scene::SceneNode::stroked_path(path.clone(), *stroke));
    }
}

impl NodePaintContext<'_> {
    /// Inserts a scene node — reusing the previous frame's slot when one is
    /// available at this position, allocating a fresh one otherwise. The id
    /// list is updated in place: a reused slot already holds the id being
    /// written (positional reuse), so only the append case touches memory.
    fn push(&mut self, mut node: floom_scene::SceneNode) {
        for &prop in &self.composite {
            apply_composite(&mut node, prop);
        }
        if self.used < self.prev.len() {
            let id = self.prev[self.used];
            self.used += 1;
            self.scene.upsert(id, node);
        } else {
            let id = self.scene.insert(node);
            self.prev.push(id);
            self.used += 1;
        }
    }
}

/// Folds an overlay's entry animation into a scene node about to be written.
///
/// The scene the host builds is flat — a widget paints rects, text and images
/// with no group node above them — so both animations lower to a per-node edit:
/// a fade scales each node's own alpha, a slide offsets its own box. Because
/// the edit is per node, the sample changes every frame and `Scene::upsert`
/// damages both the old and the new box, which is what makes the motion
/// repaint without the host asking for a full frame.
///
/// `Scale` is deliberately left alone: scaling a layout node-by-node about each
/// node's own centre would pull the subtree apart, and scaling it about a shared
/// origin needs the group node the host does not build.
///
/// Images used to be skipped alongside it, and the old comment gave the same
/// structural reason for both. Only `Scale`'s reason was structural — an image
/// simply had nowhere to put an alpha, which is a missing field rather than a
/// missing shape. `SceneNode::Image` now carries one, so an overlay holding a
/// picture fades with the rest of it instead of staying opaque over a dimming
/// background.
fn compose_transform(
    existing: Option<floom_types::Transform>,
    m: floom_types::Transform,
) -> floom_types::Transform {
    match existing {
        Some(acc) => acc.multiply(m),
        None => m,
    }
}

fn apply_composite(node: &mut floom_scene::SceneNode, prop: CompositorProp) {
    match prop {
        CompositorProp::Opacity(alpha) => {
            let alpha = alpha.clamp(0.0, 1.0);
            let scale = |c: u8| (c as f64 * alpha).round().clamp(0.0, 255.0) as u8;
            match node {
                // A gradient fades the way a solid does — every stop scales,
                // so the ramp stays a ramp instead of collapsing to its ends
                // (see `Brush::with_alpha_factor`).
                SceneNode::Rect { fill, .. } => *fill = fill.with_alpha_factor(alpha),
                SceneNode::Image { alpha, .. } => *alpha = scale(*alpha),
                SceneNode::Text { color, .. } => color.a = scale(color.a),
                // A self-drawn node fades through *both* of its inks: a chart
                // line is stroked with no fill, a chart area filled with no
                // stroke, and a shape that has both must dim as one thing.
                SceneNode::Path { fill, stroke, .. } => {
                    if let Some(fill) = fill {
                        *fill = fill.with_alpha_factor(alpha);
                    }
                    if let Some(stroke) = stroke {
                        stroke.color.a = scale(stroke.color.a);
                    }
                }
                // A clip is not painted, so it has nothing to dim. Leaving it
                // alone is the correct answer rather than an omission: dimming
                // a window would *reveal* what it was hiding.
                SceneNode::ClipPush { .. } | SceneNode::ClipPop => {}
                SceneNode::GroupPush { .. } | SceneNode::GroupPop => {}
                // Nothing to dim here either: a filter scope is a window over
                // content, and a backdrop filter reads pixels that are already
                // dimmed by their own nodes.
                SceneNode::FilterPush { .. } | SceneNode::FilterPop => {}
                SceneNode::BackdropFilter { .. } => {}
                // A shader's output is generated at raster time from its own
                // params, which a generic compositor cannot rewrite per alpha
                // (they are effect-specific, not a dimmable ink). Like a filter
                // scope, there is nothing to dim here.
                SceneNode::Shader { .. } => {}
            }
        }
        CompositorProp::TranslateX(dx) => match node {
            SceneNode::Rect { bounds, .. } | SceneNode::Image { bounds, .. } => {
                bounds.x0 += dx;
                bounds.x1 += dx;
            }
            SceneNode::Text { origin, .. } => origin.x += dx,
            // A path's geometry is its own coordinates, so it moves by moving
            // its points. A clip is a window, and a window that stayed behind
            // while its content slid would cut the content in half.
            SceneNode::Path { path, .. } => *path = std::sync::Arc::new(path.translate(dx, 0.0)),
            SceneNode::ClipPush { region } => *region = region.translate(dx, 0.0),
            _ => {}
        },
        CompositorProp::TranslateY(dy) => match node {
            SceneNode::Rect { bounds, .. } | SceneNode::Image { bounds, .. } => {
                bounds.y0 += dy;
                bounds.y1 += dy;
            }
            SceneNode::Text { origin, .. } => origin.y += dy,
            SceneNode::Path { path, .. } => *path = std::sync::Arc::new(path.translate(0.0, dy)),
            SceneNode::ClipPush { region } => *region = region.translate(0.0, dy),
            _ => {}
        },
        // Scale and rotate are *not* folded per node — scaling each rect about
        // its own centre would tear the picture apart. The paint walk brackets
        // the subtree in a `GroupPush` scope instead (see
        // `paint_subtree`/`node_group_transform`), so the payloads stay
        // untouched here and every backend applies one transform.
        CompositorProp::Scale(_) => {}
        CompositorProp::Rotate(_) => {}
    }
}

/// The overlay half of a dispatch: where requests go, who is asking, and the
/// counter that mints their handles.
struct OverlayDispatch<'a> {
    commands: &'a mut Vec<OverlayCommand>,
    next_handle: &'a mut u64,
    owner: ElementId,
}

impl OverlayDispatch<'_> {
    /// Reserves the next handle. Reserving up front (rather than letting the
    /// manager hand one back later) is what lets the content factory receive
    /// its handle synchronously, and therefore lets the content close itself
    /// without reaching back into the opener.
    fn mint(&mut self) -> OverlayHandle {
        let handle = OverlayHandle::new(*self.next_handle);
        *self.next_handle += 1;
        handle
    }
}

struct DispatchQueues<'a> {
    /// Elements that asked for focus during this dispatch, applied by the host
    /// once the widget's `event` has returned and released the node borrow.
    focus_requests: &'a mut Vec<ElementId>,
    /// Scroll offsets a widget asked for, in the same deferred shape.
    scroll_requests: &'a mut Vec<(ElementId, ScrollOffset)>,
    /// Overlay work queued by the event — opening mounts a subtree, which needs
    /// the whole host mutably, so it cannot happen inside the dispatch.
    overlay_commands: &'a mut Vec<OverlayCommand>,
}

struct NodeEventContext<'a> {
    target: ElementId,
    bounds: Rect,
    event: Event,
    focus_requests: &'a mut Vec<ElementId>,
    /// This widget's measured content extent, for the widgets that asked to be
    /// told — see [`UpdateContext::content_size`]. A wheel event arrives here
    /// rather than in `update`, and a scrolling container needs the extent to
    /// clamp against.
    content_size: Option<Size>,
    /// Scroll offsets this widget asked for, drained by the host after the event
    /// returns.
    ///
    /// The same shape as `focus_requests`: a widget cannot reach the host while
    /// its own `event` is on the stack, so it records an intent and the host
    /// applies it once the borrow ends. Encoding the request as a `(ElementId,
    /// ScrollOffset)` pair rather than letting a widget write straight to a node
    /// is what keeps the offset off the tree structure — a widget that could
    /// reach its ancestors could scroll something it does not contain.
    scroll_requests: &'a mut Vec<(ElementId, ScrollOffset)>,
    /// Where the focused composing text widget wants the IME popup anchored.
    ime_caret: &'a mut Option<Rect>,
    overlays: OverlayDispatch<'a>,
}

impl EventContext for NodeEventContext<'_> {
    fn target(&self) -> ElementId {
        self.target
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn cursor_position(&self) -> Option<Point> {
        match &self.event {
            Event::Pointer(pointer) => Some(pointer.position),
            Event::Scroll(scroll) => Some(scroll.position),
            Event::Key(_) => None,
            Event::Ime(_) => None,
        }
    }

    fn event(&self) -> Event {
        self.event.clone()
    }

    fn set_ime_cursor_area(&mut self, area: Option<Rect>) {
        *self.ime_caret = area;
    }

    fn request_focus(&mut self, id: ElementId) {
        self.focus_requests.push(id);
    }

    fn request_scroll(&mut self, offset: ScrollOffset) {
        self.scroll_requests.push((self.target, offset));
    }

    fn content_size(&mut self, _id: ElementId) -> Option<Size> {
        self.content_size
    }

    fn open_overlay(&mut self, request: OverlayRequest) -> bool {
        let handle = self.overlays.mint();
        let owner = self.overlays.owner;
        // Mint the handle and build the content now, so the request that lands
        // in the queue is complete and the content already knows how to close
        // itself. Nothing is mounted yet — the host does that once the event is
        // over and it can borrow itself again.
        let content = (request.content)(handle);
        self.overlays.commands.push(OverlayCommand::Open {
            handle,
            owner,
            kind: request.kind,
            modal: request.modal,
            bounds: request.bounds,
            content,
        });
        true
    }

    fn close_overlay(&mut self, handle: OverlayHandle) -> bool {
        self.overlays.commands.push(OverlayCommand::Close(handle));
        true
    }
}
