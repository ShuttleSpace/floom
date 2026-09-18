//! Headless app shell for Floem 2.0.
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

use floem_accessibility::{SemanticsNode, SemanticsTree};
use floem_animation::{AnimId, AnimKind, AnimationScheduler};
use floem_core::{
    Align, Axis, Child, ChildKey, CreateContext, ElementId, Event, EventContext, LayoutRequest,
    NativeGrant, NativeRequest, NativeSnapshot, OverlayHandle, OverlayKind, OverlayRequest,
    PaintContext, Point, PointerPhase, Position, Rect, ResourceId, SemanticsHint, SemanticsRole,
    Size, UpdateContext,
};
use floem_focus::{FocusDirection, FocusManager, FocusScopeId};
use floem_layout::{
    AlignItems as LAlign, FlexDirection, JustifyContent, LayoutEngine, LayoutNode, LayoutStyle,
    MeasureFn, Position as LPosition,
};
use floem_native::{
    InputMode, NativeCompositionMode, NativeError, NativeSlot, NativeState, NativeViewController,
    NativeViewId, NativeViewKind, NativeViewPool, NativeViewProvider,
};
use floem_overlay::{CompositorProp, OverlayManager};
use floem_runtime::{EventRouter, Hit, HitRegion, RouteStop};
use floem_runtime_core::IdAllocator;
use floem_scene::{Resource, ResourceKind, ResourceRegistry, Scene, SceneNodeId};
use floem_text::{LayoutCache, TextAlign, TextLayoutOptions, TextShaper};
use floem_types::NativeCapture;
use floem_types::ScrollEvent;
use floem_widgets::WidgetAny;

/// Stable handle to a mounted widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameStats {
    pub updated: usize,
    pub painted: usize,
    pub layout_nodes: usize,
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
    /// back to this node through [`floem_core::Widget::overlay_closed`] when
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

impl floem_core::Widget for Retired {
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
    /// Interned glyph runs, keyed by (text, font size), so frames reuse
    /// resources and shaping work across frames.
    glyph_cache: HashMap<(String, u32), ResourceId>,
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
    focus: FocusManager,
    scope: FocusScopeId,
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

fn to_a11y_role(role: SemanticsRole) -> floem_accessibility::SemanticsRole {
    use SemanticsRole as C;
    use floem_accessibility::SemanticsRole as A;
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

fn to_a11y_state(state: floem_core::SemanticsState) -> floem_accessibility::SemanticsState {
    use floem_accessibility::SemanticsState as A;
    use floem_core::SemanticsState as C;
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
    }
}

fn align_to_align_items(align: Align) -> LAlign {
    match align {
        Align::Stretch => LAlign::Stretch,
        Align::Start => LAlign::Start,
        Align::Center => LAlign::Center,
        Align::End => LAlign::End,
    }
}

/// Lowers a composite child's [`LayoutRequest`] into the engine's [`LayoutStyle`].
///
/// This is the heart of engine neutrality: `floem-core` never names a concrete
/// layout type, so the lowering happens here, where both vocabularies are in
/// scope.
fn request_to_style(req: &LayoutRequest) -> LayoutStyle {
    let mut style = LayoutStyle::default();
    style.width = req.width;
    style.height = req.height;
    style.flex_grow = req.flex_grow.unwrap_or(0.0);
    if let Some(axis) = req.axis {
        style.flex_direction = match axis {
            Axis::Horizontal => FlexDirection::Row,
            Axis::Vertical => FlexDirection::Column,
        };
    }
    if let Some(gap) = req.gap {
        style.gap = gap;
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
        style.inset = floem_layout::AbsoluteInset {
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
        }
        if base.height.is_none() {
            base.height = req.height;
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
        if let Some(position) = req.position {
            base.position = match position {
                Position::Relative => LPosition::Relative,
                Position::Absolute => LPosition::Absolute,
            };
        }
        if let Some(inset) = req.inset {
            base.inset = floem_layout::AbsoluteInset {
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

    let Some(text) = widget.as_any().downcast_ref::<floem_widgets::Text>() else {
        return (style, None);
    };
    if pinned_width && pinned_height {
        return (style, None);
    }
    let state = Arc::new(Mutex::new(TextMeasure {
        text: text.content().to_string(),
        font_size: text.font_size(),
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
            shaper: TextShaper::new(),
            text_layout_cache: LayoutCache::new(),
            pending_scene_removals: Vec::new(),
            semantics: SemanticsTree::new(),
            focus,
            scope,
            overlay: OverlayManager::new(),
            overlays: Vec::new(),
            overlay_commands: Vec::new(),
            next_overlay_handle: 0,
            ime_caret: None,
            anim_clock: 0.0,
            overlay_animating: false,
            native_anim: AnimationScheduler::new(),
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
    /// Defaults to [`floem_text::DEFAULT_LAYOUT_CACHE_CAPACITY`]. Lower it for a
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
        NodeId(index)
    }

    pub fn mount(
        &mut self,
        parent: NodeId,
        style: LayoutStyle,
        widget: impl WidgetAny + 'static,
    ) -> NodeId {
        assert!(parent.0 < self.nodes.len(), "unknown parent node");
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
    /// [`EventContext::open_overlay`]: floem_core::EventContext::open_overlay
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
        if let Some(owner) = owner {
            if let Some(owner_index) = self.index_of(owner) {
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
        }
        self.content_dirty = true;
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

        if let Some(owner) = slot.owner {
            if let Some(index) = self.index_of(owner) {
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
            }
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
            style.width = Some(bounds.width() as f32);
            style.height = Some(bounds.height() as f32);
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
                    let m = state.lock().unwrap();
                    if m.text.is_empty() || m.font_size <= 0.0 {
                        return Size::ZERO;
                    }
                    // `avail.width` is the box this leaf actually landed in —
                    // wrap against it; `f64::INFINITY` (unconstrained) falls
                    // through to the unwrapped extent.
                    floem_text::measure_text(&m.text, m.font_size, avail.width)
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
            dirty: true,
            children_dirty: true,
            dead: false,
            is_overlay: false,
            native: None,
            native_hidden: false,
            scene_nodes: Vec::new(),
            text_measure,
        });
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
        // Re-fetch after `reconcile`: it may have mounted new children.
        let children = self.nodes[index].children.clone();
        for child in children {
            self.reconcile_subtree(child);
        }
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
    }

    /// Marks `index` and its whole subtree dead, releasing layout, hit-test and
    /// focus registrations. The node's `NodeId` stays valid (we never shift the
    /// vector), so other live nodes' references are unaffected.
    fn unmount_subtree(&mut self, index: usize) {
        if self.nodes[index].dead {
            return;
        }
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
                .extend(self.nodes[i].scene_nodes.drain(..));
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
            || !self.pending_scene_removals.is_empty()
            || !self.scene.damage().is_empty()
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
        let animating = overlays || native;
        self.overlay_animating = animating;
        animating
    }

    /// The host's animation clock, in seconds since the first frame.
    pub fn anim_clock(&self) -> f64 {
        self.anim_clock
    }

    /// Whether any overlay is still playing its entry animation.
    pub fn overlay_animating(&self) -> bool {
        self.overlay_animating
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

    // --- viewport ----------------------------------------------------------

    pub fn set_size(&mut self, size: Size) {
        if self.size != size {
            self.size = size;
            self.layout_dirty = true;
        }
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
        // nothing.
        self.tick_animations(NOMINAL_FRAME_SECS);

        let mut retext = Vec::new();
        let viewport = Rect::from_origin_size(Point::ZERO, self.size);
        // Overlay requests a widget makes from `update` — a composite flipping
        // `present` on a state change, with no event to react to — are queued
        // here and drained once the whole tree has had its turn, for exactly the
        // reason the event path queues them: opening one mounts a subtree and
        // needs the host to itself while the asking widget is borrowed out of it.
        let mut overlay_commands: Vec<OverlayCommand> = Vec::new();
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
                bounds: node.bounds,
                viewport,
                snapshot,
                overlays: Some(UpdateOverlaySink {
                    commands: &mut overlay_commands,
                    next_handle: &mut self.next_overlay_handle,
                    owner: node.id,
                }),
                native,
            };
            node.widget.update(&mut cx, node.id);
            node.dirty = cx.dirty;
            node.children_dirty |= cx.children_dirty;
            if cx.dirty {
                stats.updated += 1;
                // Measure-driven text leaves must be re-measured before layout
                // when their content changed — otherwise the frozen extent
                // (and the stale glyph resource) survive another frame.
                if node.text_measure.is_some() {
                    retext.push(index);
                }
            }
        }
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
        }
        // Overlays lay out against the viewport independently of the main tree.
        self.layout_overlays();
        stats.layout_nodes = self.layout.node_count();

        // Participants are positioned from the layout that just ran, and their
        // hit regions depend on it, so this is the earliest it can happen.
        self.sync_native(&mut stats);

        self.paint(&mut stats);
        self.collect_semantics();

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
            .downcast_ref::<floem_widgets::Text>()
        else {
            return;
        };
        let mut m = state.lock().unwrap();
        if m.text != text.content() || m.font_size != text.font_size() {
            m.text = text.content().to_string();
            m.font_size = text.font_size();
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
            root_style.width = Some(self.size.width as f32);
        }
        if root_style.height.is_none() {
            root_style.height = Some(self.size.height as f32);
        }
        self.layout.set_style(root_node, root_style);
        self.layout.compute(root_node, self.size);
        for index in 0..self.nodes.len() {
            if self.nodes[index].dead {
                continue;
            }
            // Overlays are laid out (and positioned) separately, against the
            // viewport, so the main-tree pass must not also size them.
            if self.nodes[index].is_overlay || self.is_in_overlay(index) {
                continue;
            }
            let output = self.layout.layout(self.nodes[index].layout_node);
            self.nodes[index].bounds = output.rect;
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
                    bounds: output.rect,
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

        for index in 0..self.nodes.len() {
            // Overlay subtrees are painted afterwards, on top of the main tree.
            if self.nodes[index].is_overlay || self.is_in_overlay(index) {
                continue;
            }
            if self.nodes[index].dead {
                // Unmount already pends these; drain defensively so a dead
                // node can never keep a phantom scene node alive.
                for id in self.nodes[index].scene_nodes.drain(..) {
                    self.scene.remove(id);
                }
                continue;
            }
            // A frozen participant's tween rides along here, on the same
            // channel an overlay's entry animation uses.
            let composite = self.frozen_composite(index);
            self.paint_node(index, composite, stats);
        }

        // Overlays paint last, so their content sits above everything beneath
        // it in z — this is what lets a popup escape its parent's clip. Each
        // carries the value its entry animation sampled this frame, which the
        // paint context folds into every node it writes.
        let mut overlay_nodes: Vec<(usize, Option<CompositorProp>)> = Vec::new();
        for slot in &self.overlays {
            if !self.nodes[slot.node].dead {
                overlay_nodes.push((slot.node, self.overlay.composite(slot.id)));
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
        composite: Option<CompositorProp>,
        stats: &mut FrameStats,
    ) {
        let prev = std::mem::take(&mut self.nodes[index].scene_nodes);
        let mut next = Vec::new();
        let leftover;
        {
            let node = &self.nodes[index];
            let mut cx = NodePaintContext {
                scene: &mut self.scene,
                resources: &mut self.resources,
                glyphs: &mut self.glyph_cache,
                shaper: &mut self.shaper,
                layout_cache: &mut self.text_layout_cache,
                composite,
                prev,
                used: 0,
                next: &mut next,
                bounds: node.bounds,
            };
            node.widget.paint(&mut cx, node.id);
            let (painted, stale) = cx.finish();
            next = painted;
            leftover = stale;
        }
        for id in leftover {
            self.scene.remove(id);
        }
        self.nodes[index].scene_nodes = next;
        stats.painted += 1;
    }

    /// Pre-order paint of an overlay subtree (parent before its children),
    /// applying the overlay's entry animation to every node it writes.
    fn paint_overlay_subtree(
        &mut self,
        index: usize,
        composite: Option<CompositorProp>,
        stats: &mut FrameStats,
    ) {
        if self.nodes[index].dead {
            return;
        }
        self.paint_node(index, composite, stats);
        let children = self.nodes[index].children.clone();
        for child in children {
            self.paint_overlay_subtree(child, composite, stats);
        }
    }

    fn collect_semantics(&mut self) {
        self.semantics = SemanticsTree::new();
        for index in 0..self.nodes.len() {
            if self.nodes[index].dead {
                continue;
            }
            let node = &self.nodes[index];
            let hint = node.widget.semantics_hint();
            let children: Vec<ElementId> = node
                .children
                .iter()
                .copied()
                .filter(|&child| !self.nodes[child].dead)
                .map(|child| self.nodes[child].id)
                .collect();

            let publishes_itself = hint.as_ref().map_or(false, |hint| {
                hint.role != SemanticsRole::None || hint.label.is_some()
            });
            if !publishes_itself && children.is_empty() {
                continue;
            }

            // A purely structural node with semantic descendants becomes a
            // `Group` so the assistive-technology tree keeps its hierarchy.
            let role = match hint.as_ref().map(|hint| hint.role) {
                Some(SemanticsRole::None) => floem_accessibility::SemanticsRole::Group,
                Some(role) => to_a11y_role(role),
                None => floem_accessibility::SemanticsRole::Group,
            };
            let state = hint
                .as_ref()
                .map(|hint| to_a11y_state(hint.state))
                .unwrap_or_else(floem_accessibility::SemanticsState::empty);

            let mut semantics_node = SemanticsNode::new(role).with_state(state);
            if let Some(label) = hint.and_then(|hint| hint.label) {
                semantics_node = semantics_node.with_label(label);
            }
            for child in children {
                semantics_node = semantics_node.with_child(child);
            }
            self.semantics.set_node(node.id, semantics_node);
        }
        if let Some(root) = self.root {
            self.semantics.set_root(self.nodes[root].id);
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
        let mut overlay_commands: Vec<OverlayCommand> = Vec::new();

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
                &mut focus_requests,
                &mut overlay_commands,
            ) {
                handled = true;
                break;
            }
        }

        for id in focus_requests {
            self.set_focus(id);
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
        focus_requests: &mut Vec<ElementId>,
        overlay_commands: &mut Vec<OverlayCommand>,
    ) -> bool {
        let node = &mut self.nodes[index];
        let mut cx = NodeEventContext {
            target: stop.id,
            bounds: node.bounds,
            event,
            focus_requests,
            ime_caret: &mut self.ime_caret,
            overlays: OverlayDispatch {
                commands: overlay_commands,
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

    /// Resolves a pointer target, letting an open overlay take priority over the
    /// main tree. A modal overlay returns its id even for points outside its
    /// content (the scrim), and a hit there falls through to the overlay root
    /// so the overlay can absorb or act on the dismissal.
    fn resolve_pointer(&self, point: Point) -> Option<Hit> {
        if let Some(overlay_id) = self.overlay.hit_test(point) {
            if let Some(node) = self.overlay_node(overlay_id) {
                let target = self
                    .hit_test_overlay_subtree(node, point)
                    .unwrap_or(self.nodes[node].id);
                return Some(Hit {
                    target,
                    native: None,
                });
            }
        }
        self.router.resolve_target(point)
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
                entry.composite = sample.compositor;
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
    fn frozen_composite(&self, node: usize) -> Option<CompositorProp> {
        self.frozen
            .iter()
            .find(|f| f.node == node)
            .and_then(|f| f.composite)
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
                    .map_or(false, |index| !self.nodes[index].dead);
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
            .filter(|p| self.nodes.get(p.node).map_or(true, |node| node.dead))
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
        let modal_up = self.overlay.top().map_or(false, |top| top.modal);
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
            let needs_crop = clip.map_or(false, |clip| !clip.contains_rect(bounds));
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
        if let Some(previous) = self.focus.focused() {
            self.clear_focus_flag(previous);
        }
        self.focus.focus(id);
        self.set_focus_flag(id, true);
        self.yield_native_focus(id);
    }

    pub fn focus_next(&mut self) {
        self.move_focus(FocusDirection::Forward);
    }

    pub fn focus_prev(&mut self) {
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
    /// [`floem_widgets::Image`] widget renders from.
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
/// Only the controls know they own an `InteractionState`, so the downcast is
/// tried per concrete control type. Widgets that ignore focus simply no-op.
fn set_widget_focused(widget: &mut dyn WidgetAny, focused: bool) {
    macro_rules! try_focus {
        ($($ty:ty),*) => {
            $( if let Some(control) = widget.as_any_mut().downcast_mut::<$ty>() {
                control.set_focused(focused);
                return;
            } )*
        };
    }
    try_focus!(
        floem_widgets::Button,
        floem_widgets::TextField,
        floem_widgets::Checkbox,
        floem_widgets::Radio,
        floem_widgets::Switch,
        floem_widgets::Slider,
        floem_widgets::Select
    );
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
    /// The updating node's box (from the previous frame — `update` runs before
    /// layout), so a composite can anchor an overlay to itself.
    bounds: Rect,
    /// The host viewport, for a composite whose overlay spans the window.
    viewport: Rect,
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
            bounds,
            viewport,
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

struct NodePaintContext<'a> {
    scene: &'a mut Scene,
    resources: &'a mut ResourceRegistry,
    glyphs: &'a mut HashMap<(String, u32), ResourceId>,
    shaper: &'a mut TextShaper,
    layout_cache: &'a mut LayoutCache,
    /// The overlay entry animation this node is being painted under, if any.
    /// Applied to every scene node the context writes, so a whole overlay
    /// subtree fades or slides as one.
    composite: Option<CompositorProp>,
    /// Last frame's scene node ids for this widget. Painting reuses them
    /// positionally: `upsert` on a match (no damage when the payload is
    /// unchanged), `insert` once the previous nodes run out.
    prev: Vec<SceneNodeId>,
    used: usize,
    /// Scene node ids painted this frame; written back to the node afterwards.
    next: &'a mut Vec<SceneNodeId>,
    bounds: Rect,
}

impl NodePaintContext<'_> {
    /// Consumes the context: returns the ids painted this frame plus the
    /// previous ids that were never reused (retired by the caller, which
    /// damages their old bounds).
    fn finish(self) -> (Vec<SceneNodeId>, Vec<SceneNodeId>) {
        let leftover = self.prev[self.used..].to_vec();
        (std::mem::take(self.next), leftover)
    }

    /// Interns a shaped glyph run, reusing the resource across frames.
    ///
    /// The cache key is (text, font size): the same string at two sizes has
    /// different glyph ids and positions, so the size must be part of the key.
    fn intern_glyphs(
        &mut self,
        text: &str,
        font_size: f32,
        runs: Vec<floem_text::GlyphRun>,
    ) -> ResourceId {
        let key = (text.to_string(), font_size.to_bits());
        if let Some(id) = self.glyphs.get(&key) {
            return *id;
        }
        let id = self.resources.insert(Resource {
            kind: ResourceKind::GlyphRun { runs },
        });
        self.glyphs.insert(key, id);
        id
    }
}

impl PaintContext for NodePaintContext<'_> {
    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn fill_rect(&mut self, rect: Rect, color: floem_types::Color) {
        self.push(floem_scene::SceneNode::Rect {
            bounds: rect,
            fill: color,
            radius: 0.0,
        });
    }

    fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: floem_types::Color) {
        self.push(floem_scene::SceneNode::Rect {
            bounds: rect,
            fill: color,
            radius: radius.max(0.0),
        });
    }

    fn fill_text(&mut self, origin: Point, text: &str, size: f32, color: floem_types::Color) {
        if text.is_empty() || size <= 0.0 {
            return;
        }
        // Shape within the laid-out box so wrapped text paints as wrapped:
        // the measure callback sized this node against the same width.
        let max_width = if self.bounds.width() > 0.0 {
            self.bounds.width()
        } else {
            f64::INFINITY
        };
        let options = TextLayoutOptions {
            max_width,
            font_size: size,
            align: TextAlign::Start,
        };
        // Reuse a previously shaped result when the text + layout options are
        // unchanged: shaping is the expensive part (parse / script segmentation
        // / glyph placement), and reusing the cached `GlyphRun`s keeps their
        // font data stable so the downstream glyph raster cache keeps hitting.
        let shaper = &mut *self.shaper;
        let (runs, laid_out_size) = {
            let laid = self
                .layout_cache
                .get_or_insert_with(text, &options, || shaper.shape(text, &options));
            (laid.runs.clone(), laid.size)
        };
        let glyphs = self.intern_glyphs(text, size, runs);
        self.push(floem_scene::SceneNode::Text {
            origin,
            size: laid_out_size,
            glyphs,
            color,
        });
    }

    fn draw_image(&mut self, bounds: Rect, image: ResourceId) {
        self.push(floem_scene::SceneNode::image(bounds, image));
    }
}

impl NodePaintContext<'_> {
    /// Inserts a scene node — reusing the previous frame's slot when one is
    /// available at this position, allocating a fresh one otherwise.
    fn push(&mut self, mut node: floem_scene::SceneNode) {
        if let Some(prop) = self.composite {
            apply_composite(&mut node, prop);
        }
        let id = if self.used < self.prev.len() {
            let id = self.prev[self.used];
            self.used += 1;
            self.scene.upsert(id, node);
            id
        } else {
            self.scene.insert(node)
        };
        self.next.push(id);
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
fn apply_composite(node: &mut floem_scene::SceneNode, prop: CompositorProp) {
    match prop {
        CompositorProp::Opacity(alpha) => {
            let alpha = alpha.clamp(0.0, 1.0);
            let scale = |c: u8| (c as f64 * alpha).round().clamp(0.0, 255.0) as u8;
            match node {
                floem_scene::SceneNode::Rect { fill, .. } => fill.a = scale(fill.a),
                floem_scene::SceneNode::Image { alpha, .. } => *alpha = scale(*alpha),
                floem_scene::SceneNode::Text { color, .. } => color.a = scale(color.a),
                floem_scene::SceneNode::Group { .. } => {}
            }
        }
        CompositorProp::TranslateX(dx) => match node {
            floem_scene::SceneNode::Rect { bounds, .. }
            | floem_scene::SceneNode::Image { bounds, .. } => {
                bounds.x0 += dx;
                bounds.x1 += dx;
            }
            floem_scene::SceneNode::Text { origin, .. } => origin.x += dx,
            _ => {}
        },
        CompositorProp::TranslateY(dy) => match node {
            floem_scene::SceneNode::Rect { bounds, .. }
            | floem_scene::SceneNode::Image { bounds, .. } => {
                bounds.y0 += dy;
                bounds.y1 += dy;
            }
            floem_scene::SceneNode::Text { origin, .. } => origin.y += dy,
            _ => {}
        },
        CompositorProp::Scale(_) => {}
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

struct NodeEventContext<'a> {
    target: ElementId,
    bounds: Rect,
    event: Event,
    focus_requests: &'a mut Vec<ElementId>,
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
