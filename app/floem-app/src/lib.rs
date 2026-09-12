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

use std::collections::HashMap;

use floem_accessibility::{SemanticsNode, SemanticsTree};
use floem_core::{
    CreateContext, ElementId, Event, EventContext, PaintContext, Point, Rect, ResourceId, Size,
    UpdateContext,
};
use floem_focus::{FocusDirection, FocusManager, FocusScopeId};
use floem_layout::{LayoutEngine, LayoutNode, LayoutStyle};
use floem_runtime::{EventRouter, Hit, HitRegion, RouteStop};
use floem_runtime_core::IdAllocator;
use floem_scene::{Resource, ResourceKind, ResourceRegistry, Scene, SceneNodeId};
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
    style: LayoutStyle,
    /// Tree depth, used to give deeper nodes priority in hit-testing.
    depth: u32,
    /// Set by the last `update()`; drives whether we repaint this frame.
    dirty: bool,
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
    /// Interned glyph runs, keyed by text, so frames reuse resources.
    glyph_cache: HashMap<String, ResourceId>,
    /// Scene nodes produced by the current frame, retired on the next one.
    painted_nodes: Vec<SceneNodeId>,
    semantics: SemanticsTree,
    focus: FocusManager,
    scope: FocusScopeId,
    size: Size,
    layout_dirty: bool,
    last_stats: FrameStats,
}

impl Default for WidgetHost {
    fn default() -> Self {
        Self::new()
    }
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
            painted_nodes: Vec::new(),
            semantics: SemanticsTree::new(),
            focus,
            scope,
            size: Size::new(800.0, 600.0),
            layout_dirty: true,
            last_stats: FrameStats::default(),
        }
    }

    // --- tree construction -------------------------------------------------

    pub fn mount_root(&mut self, style: LayoutStyle, widget: impl WidgetAny + 'static) -> NodeId {
        let index = self.push(None, style, widget);
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
        NodeId(self.push(Some(parent.0), style, widget))
    }

    fn push(
        &mut self,
        parent: Option<usize>,
        style: LayoutStyle,
        widget: impl WidgetAny + 'static,
    ) -> usize {
        let index = self.nodes.len();
        let id = {
            let mut cx = HostCreateContext {
                allocator: &mut self.ids,
            };
            widget.create(&mut cx)
        };
        let layout_node = self.layout.new_leaf(style.clone());
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
            None | Some(floem_accessibility::SemanticsRole::None)
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
            widget: Box::new(widget),
            layout_node,
            bounds: Rect::ZERO,
            parent,
            children: Vec::new(),
            style,
            depth,
            dirty: true,
        });
        self.layout_dirty = true;
        index
    }

    /// Borrows the concrete widget behind `node`.
    pub fn widget<T: WidgetAny + 'static>(&self, node: NodeId) -> Option<&T> {
        self.nodes.get(node.0)?.widget.as_any().downcast_ref::<T>()
    }

    /// Mutably borrows the concrete widget behind `node`.
    pub fn widget_mut<T: WidgetAny + 'static>(&mut self, node: NodeId) -> Option<&mut T> {
        self.nodes
            .get_mut(node.0)?
            .widget
            .as_any_mut()
            .downcast_mut::<T>()
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

        for index in 0..self.nodes.len() {
            let node = &mut self.nodes[index];
            let mut cx = HostUpdateContext { dirty: false };
            node.widget.update(&mut cx, node.id);
            node.dirty = cx.dirty;
            if cx.dirty {
                stats.updated += 1;
            }
        }

        if self.layout_dirty {
            self.run_layout();
            self.layout_dirty = false;
        }
        stats.layout_nodes = self.layout.node_count();

        self.paint(&mut stats);
        self.collect_semantics();

        self.last_stats = stats;
        stats
    }

    fn run_layout(&mut self) {
        let Some(root) = self.root else {
            return;
        };
        let root_node = self.nodes[root].layout_node;
        if self.size.width <= 0.0 || self.size.height <= 0.0 {
            return;
        }
        // The root fills the viewport unless it asked for an explicit size.
        // Without this a `<Container>` root shrink-wraps its content and
        // `justify_content` has no free space to distribute.
        let mut root_style = self.nodes[root].style.clone();
        let mut root_restyled = false;
        if root_style.width.is_none() {
            root_style.width = Some(self.size.width as f32);
            root_restyled = true;
        }
        if root_style.height.is_none() {
            root_style.height = Some(self.size.height as f32);
            root_restyled = true;
        }
        if root_restyled {
            self.layout.set_style(root_node, root_style);
        }
        self.layout.compute(root_node, self.size);
        for index in 0..self.nodes.len() {
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
        self.scene.begin_update();
        // Retire last frame's scene nodes: painting allocates fresh ones, so
        // without this the retained scene would grow every frame.
        let stale = std::mem::take(&mut self.painted_nodes);
        for id in stale {
            self.scene.remove(id);
        }
        for index in 0..self.nodes.len() {
            let node = &self.nodes[index];
            let mut cx = NodePaintContext {
                scene: &mut self.scene,
                resources: &mut self.resources,
                glyphs: &mut self.glyph_cache,
                painted: &mut self.painted_nodes,
                bounds: node.bounds,
            };
            node.widget.paint(&mut cx, node.id);
            stats.painted += 1;
        }
        self.scene.finish_update();
    }

    fn collect_semantics(&mut self) {
        self.semantics = SemanticsTree::new();
        for index in 0..self.nodes.len() {
            let node = &self.nodes[index];
            let hint = node.widget.semantics_hint();
            let children: Vec<ElementId> = node
                .children
                .iter()
                .map(|child| self.nodes[*child].id)
                .collect();

            let publishes_itself = hint.as_ref().map_or(false, |hint| {
                hint.role != floem_accessibility::SemanticsRole::None || hint.label.is_some()
            });
            if !publishes_itself && children.is_empty() {
                continue;
            }

            // A purely structural node with semantic descendants becomes a
            // `Group` so the assistive-technology tree keeps its hierarchy.
            let role = hint
                .as_ref()
                .map(|hint| hint.role)
                .filter(|role| *role != floem_accessibility::SemanticsRole::None)
                .unwrap_or(floem_accessibility::SemanticsRole::Group);
            let state = hint
                .as_ref()
                .map_or_else(floem_accessibility::SemanticsState::empty, |hint| {
                    hint.state
                });

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
            Event::Pointer(pointer) => self.router.resolve_target(pointer.position),
            Event::Scroll(ScrollEvent { position, .. }) => self.router.resolve_target(position),
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

        for stop in route {
            let Some(index) = self.index_of(stop.id) else {
                continue;
            };
            if self.dispatch_at(index, stop, event, &mut focus_requests) {
                handled = true;
                break;
            }
        }

        for id in focus_requests {
            self.set_focus(id);
        }
        handled
    }

    fn dispatch_at(
        &mut self,
        index: usize,
        stop: RouteStop,
        event: Event,
        focus_requests: &mut Vec<ElementId>,
    ) -> bool {
        let node = &mut self.nodes[index];
        let mut cx = NodeEventContext {
            target: stop.id,
            bounds: node.bounds,
            event,
            focus_requests,
        };
        node.widget.event(&mut cx, stop.id)
    }

    fn index_of(&self, id: ElementId) -> Option<usize> {
        self.nodes.iter().position(|node| node.id == id)
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
}

impl UpdateContext for HostUpdateContext {
    fn mark_dirty(&mut self, _id: ElementId) {
        self.dirty = true;
    }
}

struct NodePaintContext<'a> {
    scene: &'a mut Scene,
    resources: &'a mut ResourceRegistry,
    glyphs: &'a mut HashMap<String, ResourceId>,
    painted: &'a mut Vec<SceneNodeId>,
    bounds: Rect,
}

impl NodePaintContext<'_> {
    /// Inserts a scene node and records it for retirement next frame.
    fn push(&mut self, node: floem_scene::SceneNode) {
        let id = self.scene.insert(node);
        self.painted.push(id);
    }

    /// Interns `text` as a glyph run, reusing the resource across frames.
    fn intern_glyphs(&mut self, text: &str) -> ResourceId {
        if let Some(id) = self.glyphs.get(text) {
            return *id;
        }
        let id = self.resources.insert(Resource {
            kind: ResourceKind::GlyphRun {
                glyph_count: text.chars().count() as u32,
            },
        });
        self.glyphs.insert(text.to_string(), id);
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
        });
    }

    fn fill_rounded_rect(&mut self, rect: Rect, _radius: f32, color: floem_types::Color) {
        // No rounded-rect node in the retained scene yet; the radius is carried
        // alongside so backends can lower it once the node exists.
        self.push(floem_scene::SceneNode::Rect {
            bounds: rect,
            fill: color,
        });
    }

    fn fill_text(&mut self, origin: Point, text: &str, size: f32, color: floem_types::Color) {
        if text.is_empty() || size <= 0.0 {
            return;
        }
        let glyphs = self.intern_glyphs(text);
        // Advance estimate only; `floem-text` owns real shaping.
        let advance = Size::new(
            text.chars().count() as f64 * f64::from(size) * 0.6,
            f64::from(size) * 1.2,
        );
        self.push(floem_scene::SceneNode::Text {
            origin,
            size: advance,
            glyphs,
            color,
        });
    }

    fn draw_image(&mut self, bounds: Rect, image: ResourceId) {
        self.push(floem_scene::SceneNode::Image { bounds, image });
    }
}

struct NodeEventContext<'a> {
    target: ElementId,
    bounds: Rect,
    event: Event,
    focus_requests: &'a mut Vec<ElementId>,
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
}
