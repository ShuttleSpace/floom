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
use floem_core::{
    Align, Axis, Child, ChildKey, CreateContext, ElementId, Event, EventContext, LayoutRequest,
    OverlayHandle, OverlayKind, OverlayRequest, PaintContext, Point, Position, Rect, ResourceId,
    SemanticsHint, SemanticsRole, Size, UpdateContext,
};
use floem_focus::{FocusDirection, FocusManager, FocusScopeId};
use floem_layout::{
    AlignItems as LAlign, FlexDirection, JustifyContent, LayoutEngine, LayoutNode, LayoutStyle,
    MeasureFn, Position as LPosition,
};
use floem_overlay::OverlayManager;
use floem_runtime::{EventRouter, Hit, HitRegion, RouteStop};
use floem_runtime_core::IdAllocator;
use floem_scene::{Resource, ResourceKind, ResourceRegistry, Scene, SceneNodeId};
use floem_text::{TextAlign, TextLayoutOptions, TextShaper};
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
    /// Set for `Text` leaves whose width/height is not pinned: they size
    /// themselves through a taffy measure callback reading this shared state,
    /// so extent changes (content edit, container reflow) re-measure instead
    /// of freezing the mount-time measurement.
    text_measure: Option<Arc<Mutex<TextMeasure>>>,
}

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
            pending_scene_removals: Vec::new(),
            semantics: SemanticsTree::new(),
            focus,
            scope,
            overlay: OverlayManager::new(),
            overlays: Vec::new(),
            overlay_commands: Vec::new(),
            next_overlay_handle: 0,
            size: Size::new(800.0, 600.0),
            layout_dirty: true,
            content_dirty: false,
            last_stats: FrameStats::default(),
        }
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
        let id = self
            .overlay
            .push(handle, kind, self.nodes[index].id, bounds, modal, 0.0);
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
                let mut cx = HostUpdateContext {
                    dirty: false,
                    children_dirty: false,
                };
                self.nodes[owner_index]
                    .widget
                    .overlay_opened(&mut cx, owner_id, handle);
                let HostUpdateContext {
                    dirty,
                    children_dirty,
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
                let mut cx = HostUpdateContext {
                    dirty: false,
                    children_dirty: false,
                };
                self.nodes[index]
                    .widget
                    .overlay_closed(&mut cx, owner_id, id, content);
                let HostUpdateContext {
                    dirty,
                    children_dirty,
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
            let mut style = LayoutStyle::default();
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
            if let Some(pos) = self.nodes[parent].children.iter().position(|&c| c == index) {
                self.nodes[parent].children.remove(pos);
                self.nodes[parent].child_keys.remove(pos);
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
            || !self.pending_scene_removals.is_empty()
            || !self.scene.damage().is_empty()
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

        let mut retext = Vec::new();
        for index in 0..self.nodes.len() {
            if self.nodes[index].dead {
                continue;
            }
            let node = &mut self.nodes[index];
            let mut cx = HostUpdateContext {
                dirty: false,
                children_dirty: false,
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
            let z = self.nodes[index].depth as i32 * 1_000_000 + index as i32;
            self.router.hits.set_region(
                self.nodes[index].id,
                HitRegion {
                    bounds: output.rect,
                    native: None,
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
            self.paint_node(index, stats);
        }

        // Overlays paint last, so their content sits above everything beneath
        // it in z — this is what lets a popup escape its parent's clip.
        let overlay_nodes: Vec<usize> = self
            .overlays
            .iter()
            .map(|s| s.node)
            .filter(|&node| !self.nodes[node].dead)
            .collect();
        for node in overlay_nodes {
            self.paint_overlay_subtree(node, stats);
        }
        self.scene.finish_update();
    }

    /// Paints one node into the scene, reusing its previous frame's scene nodes
    /// positionally (so unchanged payloads produce no damage) and retiring the
    /// ones it no longer produces.
    fn paint_node(&mut self, index: usize, stats: &mut FrameStats) {
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

    /// Pre-order paint of an overlay subtree (parent before its children).
    fn paint_overlay_subtree(&mut self, index: usize, stats: &mut FrameStats) {
        if self.nodes[index].dead {
            return;
        }
        self.paint_node(index, stats);
        let children = self.nodes[index].children.clone();
        for child in children {
            self.paint_overlay_subtree(child, stats);
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
        let target = match event {
            // Overlays win over the main tree, front-to-back; a modal overlay
            // also swallows clicks outside its content (the scrim). The router
            // only sees the main tree, so overlay hits are resolved here first.
            Event::Pointer(pointer) => self.resolve_pointer(pointer.position),
            Event::Scroll(ScrollEvent { position, .. }) => self.resolve_pointer(position),
            Event::Key(_) => self.focus.focused().map(|target| Hit {
                target,
                native: None,
            }),
        };
        let Some(hit) = target else {
            return false;
        };
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
                event,
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

    // --- focus -------------------------------------------------------------

    pub fn focused(&self) -> Option<ElementId> {
        self.focus.focused()
    }

    pub fn set_focus(&mut self, id: ElementId) {
        if let Some(previous) = self.focus.focused() {
            self.clear_focus_flag(previous);
        }
        self.focus.focus(id);
        self.set_focus_flag(id, true);
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

struct HostUpdateContext {
    dirty: bool,
    children_dirty: bool,
}

impl UpdateContext for HostUpdateContext {
    fn mark_dirty(&mut self, _id: ElementId) {
        self.dirty = true;
    }

    fn mark_children_dirty(&mut self, _id: ElementId) {
        self.children_dirty = true;
    }
}

struct NodePaintContext<'a> {
    scene: &'a mut Scene,
    resources: &'a mut ResourceRegistry,
    glyphs: &'a mut HashMap<(String, u32), ResourceId>,
    shaper: &'a mut TextShaper,
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
        let laid = self.shaper.shape(text, &options);
        let runs = laid.runs;
        let laid_out_size = laid.size;
        let glyphs = self.intern_glyphs(text, size, runs);
        self.push(floem_scene::SceneNode::Text {
            origin,
            size: laid_out_size,
            glyphs,
            color,
        });
    }

    fn draw_image(&mut self, bounds: Rect, image: ResourceId) {
        self.push(floem_scene::SceneNode::Image { bounds, image });
    }
}

impl NodePaintContext<'_> {
    /// Inserts a scene node — reusing the previous frame's slot when one is
    /// available at this position, allocating a fresh one otherwise.
    fn push(&mut self, node: floem_scene::SceneNode) {
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
        match self.event {
            Event::Pointer(pointer) => Some(pointer.position),
            Event::Scroll(scroll) => Some(scroll.position),
            Event::Key(_) => None,
        }
    }

    fn event(&self) -> Event {
        self.event
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
