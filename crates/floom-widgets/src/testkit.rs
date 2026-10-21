//! Deterministic recorders for widget snapshot tests.
//!
//! These implement the [`floom_core`] context traits and capture what a widget
//! emits, so primitives can be asserted against a stable textual snapshot
//! without a renderer or a window. The paint recorder additionally lowers every
//! op into a real [`Scene`], which proves the widget output can be retained.

use std::any::Any;

use floom_core::{
    Color, CreateContext, ElementId, Event, EventContext, FontWeight, OverlayHandle, OverlayKind,
    PaintContext, Point, Rect, ResourceId, SemanticsContext, SemanticsRole, Size, UpdateContext,
    Widget, WidgetAny,
};
use floom_scene::{Resource, ResourceKind, ResourceRegistry, Scene, SceneNode};
use floom_types::{PointerEvent, StyledText};

/// A renderer-neutral record of a single paint operation.
#[derive(Debug, Clone, PartialEq)]
pub enum PaintOp {
    Rect {
        bounds: Rect,
        color: Color,
    },
    RoundedRect {
        bounds: Rect,
        radius: f32,
        color: Color,
    },
    Text {
        origin: Point,
        size: f32,
        text: String,
        /// How heavy the run was asked to be. Recorded because there is otherwise
        /// no way to tell a `Medium` label from a `Normal` one in a widget test:
        /// the retained scene node points at a glyph *resource*, and this
        /// recorder deliberately shapes nothing, so the request is the only
        /// observable that survives.
        weight: FontWeight,
        color: Color,
    },
    /// Text described in *ranges* rather than as one string.
    ///
    /// The ranges are the whole point of the op: a line with a highlighted
    /// range is one op with several inks, and the op log is the only place a
    /// widget test can see them — the retained scene node points at a glyph
    /// *resource*, and this recorder deliberately shapes nothing.
    StyledText {
        origin: Point,
        size: f32,
        text: String,
        /// The ranges that differ from the base style, and their inks.
        spans: Vec<(std::ops::Range<usize>, Color)>,
        weight: FontWeight,
        color: Color,
    },
    Image {
        bounds: Rect,
        resource: ResourceId,
    },
    /// A rect whose ink varies per pixel — a gradient. The op log keeps what
    /// one line can say (the colour the brush samples at the rect's centre,
    /// the colour a one-colour backend would fall back to); the whole brush
    /// lives on the retained scene node, which is where a test asserting the
    /// ramp itself has to look.
    BrushRect {
        bounds: Rect,
        radius: f32,
        sampled: Color,
    },
    /// A cast shadow: the silhouette requested of it, plus the blur and offset
    /// that carry it past that silhouette. Carries no rendered result —
    /// whether a shadow reaches a pixel is the backend's answer, not the
    /// request's — so a test asserting pixels asks a renderer, and a test
    /// asserting the ask reads this op.
    Shadow {
        bounds: Rect,
        radius: f32,
        blur: f64,
        offset_x: f64,
        offset_y: f64,
        spread: f64,
    },
    /// A filled and/or stroked vector path. `segments` is the flattened count
    /// the node carries, so two shapes that share a box are still telling
    /// apart.
    Path {
        bounds: Rect,
        segments: usize,
        fill: Option<Color>,
        stroke: Option<(f64, Color)>,
    },
    /// A filter over what has already been drawn inside `bounds`. Carries the
    /// request, not a result — whether the backdrop went soft is a renderer's
    /// answer, and a test asserting pixels asks one.
    BackdropFilter {
        bounds: Rect,
        radius_x: f64,
        radius_y: f64,
    },
}

fn fmt_rect(r: Rect) -> String {
    format!("({:.1}, {:.1}, {:.1}, {:.1})", r.x0, r.y0, r.x1, r.y1)
}

fn fmt_color(c: Color) -> String {
    format!("#{:02X}{:02X}{:02X}{:02X}", c.r, c.g, c.b, c.a)
}

impl PaintOp {
    /// Stable one-line rendering used by snapshot assertions.
    pub fn snapshot_line(&self) -> String {
        match self {
            PaintOp::Rect { bounds, color } => {
                format!("Rect {} {}", fmt_rect(*bounds), fmt_color(*color))
            }
            PaintOp::RoundedRect {
                bounds,
                radius,
                color,
            } => {
                format!(
                    "RoundedRect {} r={:.1} {}",
                    fmt_rect(*bounds),
                    radius,
                    fmt_color(*color)
                )
            }
            PaintOp::Text {
                origin,
                size,
                text,
                weight,
                color,
            } => format!(
                // The weight is appended only when it is not `Normal`: these
                // lines are compared byte-for-byte by golden snapshot tests, and
                // a trailing "Normal" on every line would churn all of them to
                // record something they did not ask about.
                "Text ({:.1}, {:.1}) {:.1} {:?}{} {}",
                origin.x,
                origin.y,
                size,
                text,
                if weight.is_normal() {
                    String::new()
                } else {
                    format!(" @{weight:?}")
                },
                fmt_color(*color)
            ),
            PaintOp::StyledText {
                origin,
                size,
                text,
                spans,
                weight,
                color,
            } => format!(
                "StyledText ({:.1}, {:.1}) {:.1} {:?} spans={:?}{} {}",
                origin.x,
                origin.y,
                size,
                text,
                spans,
                if weight.is_normal() {
                    String::new()
                } else {
                    format!(" @{weight:?}")
                },
                fmt_color(*color)
            ),
            PaintOp::BackdropFilter {
                bounds,
                radius_x,
                radius_y,
            } => format!(
                "BackdropFilter {} r=({:.1}, {:.1})",
                fmt_rect(*bounds),
                radius_x,
                radius_y
            ),
            PaintOp::Image { bounds, resource } => format!(
                "Image {} res=({}, {})",
                fmt_rect(*bounds),
                resource.index(),
                resource.generation()
            ),
            PaintOp::BrushRect {
                bounds,
                radius,
                sampled,
            } => format!(
                "BrushRect {} r={:.1} {}",
                fmt_rect(*bounds),
                radius,
                fmt_color(*sampled)
            ),
            PaintOp::Shadow {
                spread,
                bounds,
                radius,
                blur,
                offset_x,
                offset_y,
            } => format!(
                "Shadow {} r={:.1} blur={:.1} off=({:.1}, {:.1}) spread={:.1}",
                fmt_rect(*bounds),
                radius,
                blur,
                offset_x,
                offset_y,
                spread
            ),
            PaintOp::Path {
                bounds,
                segments,
                fill,
                stroke,
            } => {
                let fill = fill.map(fmt_color).unwrap_or_else(|| "none".to_string());
                let stroke = stroke
                    .map(|(w, c)| format!("{w:.1}/{}", fmt_color(c)))
                    .unwrap_or_else(|| "none".to_string());
                format!(
                    "Path {} segs={} fill={} stroke={}",
                    fmt_rect(*bounds),
                    segments,
                    fill,
                    stroke
                )
            }
        }
    }
}

/// Captures paint ops and mirrors them into a retained [`Scene`].
pub struct SceneRecorder {
    bounds: Rect,
    scene: Scene,
    registry: ResourceRegistry,
    ops: Vec<PaintOp>,
    finished: bool,
    /// The animation value a widget's paint reads back. `None` reproduces a
    /// frame with nothing running; a test sets it to simulate a frame in the
    /// middle of a declared animation.
    animation_value: Option<f64>,
}

impl SceneRecorder {
    pub fn new(bounds: Rect) -> Self {
        let mut scene = Scene::new();
        scene.begin_update();
        Self {
            bounds,
            scene,
            registry: ResourceRegistry::new(),
            ops: Vec::new(),
            finished: false,
            animation_value: None,
        }
    }

    /// Simulates a frame where an animation of this element reports `value`.
    pub fn with_animation_value(mut self, value: f64) -> Self {
        self.animation_value = Some(value);
        self
    }

    /// Ends the scene transaction. Safe to call more than once.
    pub fn finish(&mut self) {
        if !self.finished {
            self.finished = true;
            self.scene.finish_update();
        }
    }

    pub fn ops(&self) -> &[PaintOp] {
        &self.ops
    }

    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Records a path into both the op log and the retained scene.
    ///
    /// `stroke` carries only what the snapshot line shows; the retained node
    /// gets the whole style, so a test can assert on the round cap the recorder
    /// never had to print.
    fn record_path(
        &mut self,
        path: &floom_types::Path,
        fill: Option<Color>,
        stroke: Option<(f64, Color)>,
    ) {
        let node = match (fill, stroke) {
            (Some(fill), None) => SceneNode::filled_path(path.clone(), fill),
            (None, Some((width, color))) => {
                SceneNode::stroked_path(path.clone(), floom_types::Stroke::new(width, color))
            }
            _ => SceneNode::Path {
                path: std::sync::Arc::new(path.clone()),
                fill: fill.map(floom_types::Brush::from),
                stroke: stroke.map(|(width, color)| floom_types::Stroke::new(width, color)),
            },
        };
        self.scene.insert(node);
        self.ops.push(PaintOp::Path {
            bounds: path.bounds(),
            segments: path.segments().len(),
            fill,
            stroke,
        });
    }

    /// Node kind name for each retained scene node, in insertion order.
    pub fn scene_node_kinds(&self) -> Vec<&'static str> {
        self.scene
            .iter()
            .map(|(_, node)| match node {
                SceneNode::GroupPush { .. } => "GroupPush",
                SceneNode::GroupPop => "GroupPop",
                SceneNode::Rect { .. } => "Rect",
                SceneNode::Path { .. } => "Path",
                SceneNode::ClipPush { .. } => "ClipPush",
                SceneNode::ClipPop => "ClipPop",
                SceneNode::Image { .. } => "Image",
                SceneNode::Text { .. } => "Text",
                SceneNode::FilterPush { .. } => "FilterPush",
                SceneNode::FilterPop => "FilterPop",
                SceneNode::BackdropFilter { .. } => "BackdropFilter",
                SceneNode::Shader { .. } => "Shader",
            })
            .collect()
    }

    /// Newline-joined [`PaintOp::snapshot_line`]s; `"(empty)"` when nothing painted.
    pub fn snapshot(&self) -> String {
        if self.ops.is_empty() {
            return "(empty)".to_string();
        }
        self.ops
            .iter()
            .map(|op| op.snapshot_line())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl PaintContext for SceneRecorder {
    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.scene.insert(SceneNode::Rect {
            bounds: rect,
            fill: color.into(),
            radius: 0.0,
            shadow: None,
        });
        self.ops.push(PaintOp::Rect {
            bounds: rect,
            color,
        });
    }

    fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.scene.insert(SceneNode::Rect {
            bounds: rect,
            fill: color.into(),
            radius: radius.max(0.0),
            shadow: None,
        });
        self.ops.push(PaintOp::RoundedRect {
            bounds: rect,
            radius,
            color,
        });
    }

    fn fill_rect_brush(&mut self, rect: Rect, brush: &floom_types::Brush) {
        let sampled = brush.sample(rect.center());
        self.scene.insert(SceneNode::Rect {
            bounds: rect,
            fill: brush.clone(),
            radius: 0.0,
            shadow: None,
        });
        self.ops.push(PaintOp::BrushRect {
            bounds: rect,
            radius: 0.0,
            sampled,
        });
    }

    fn fill_rounded_rect_brush(&mut self, rect: Rect, radius: f32, brush: &floom_types::Brush) {
        let sampled = brush.sample(rect.center());
        self.scene.insert(SceneNode::Rect {
            bounds: rect,
            fill: brush.clone(),
            radius: radius.max(0.0),
            shadow: None,
        });
        self.ops.push(PaintOp::BrushRect {
            bounds: rect,
            radius,
            sampled,
        });
    }

    fn fill_path_brush(&mut self, path: &floom_types::Path, brush: &floom_types::Brush) {
        self.scene
            .insert(SceneNode::filled_path(path.clone(), brush.clone()));
        self.ops.push(PaintOp::Path {
            bounds: path.bounds(),
            segments: path.segments().len(),
            fill: brush.solid(),
            stroke: None,
        });
    }

    fn draw_shadow(&mut self, rect: Rect, radius: f32, shadow: &floom_types::Shadow) {
        self.scene.insert(SceneNode::Rect {
            bounds: rect,
            fill: Color::TRANSPARENT.into(),
            radius: radius.max(0.0),
            shadow: Some(*shadow),
        });
        self.ops.push(PaintOp::Shadow {
            bounds: rect,
            radius,
            blur: shadow.blur,
            offset_x: shadow.offset_x,
            offset_y: shadow.offset_y,
            spread: shadow.spread,
        });
    }

    fn fill_styled_text(&mut self, origin: Point, styled: &StyledText) {
        // No shaping happens here (`floom-text` owns that), so the retained
        // node gets a placeholder advance box just to keep a bounds hint — the
        // same placeholder the plain text op gets.
        let advance = Size::new(
            styled.text.chars().count() as f64 * f64::from(styled.base.size) * 0.5,
            f64::from(styled.base.size) * 1.2,
        );
        let glyphs = self.registry.insert(Resource {
            kind: ResourceKind::glyph_runs(Vec::new()),
        });
        self.scene.insert(SceneNode::Text {
            origin,
            size: advance,
            glyphs,
            color: styled.base.color,
        });
        self.ops.push(PaintOp::StyledText {
            origin,
            size: styled.base.size,
            text: styled.text.clone(),
            spans: styled
                .spans
                .iter()
                .map(|span| (span.range.clone(), span.style.color))
                .collect(),
            weight: styled.base.weight,
            color: styled.base.color,
        });
    }

    fn fill_text(
        &mut self,
        origin: Point,
        text: &str,
        size: f32,
        weight: FontWeight,
        color: Color,
    ) {
        // No shaping happens here (`floom-text` owns that); an empty run list
        // keeps the resource present without pulling a text dependency into
        // the widget layer.
        let glyphs = self.registry.insert(Resource {
            kind: ResourceKind::glyph_runs(Vec::new()),
        });
        // No shaping happens here (`floom-text` owns that), so the retained
        // node gets a placeholder advance box just to keep a bounds hint.
        let advance = Size::new(
            text.chars().count() as f64 * f64::from(size) * 0.5,
            f64::from(size) * 1.2,
        );
        self.scene.insert(SceneNode::Text {
            origin,
            size: advance,
            glyphs,
            color,
        });
        self.ops.push(PaintOp::Text {
            origin,
            size,
            text: text.to_string(),
            weight,
            color,
        });
    }

    fn fill_path(&mut self, path: &floom_types::Path, color: Color) {
        self.record_path(path, Some(color), None);
    }

    fn stroke_path(&mut self, path: &floom_types::Path, stroke: &floom_types::Stroke) {
        self.record_path(path, None, Some((stroke.width, stroke.color)));
    }

    fn draw_image(&mut self, bounds: Rect, image: ResourceId) {
        self.scene.insert(SceneNode::image(bounds, image));
        self.ops.push(PaintOp::Image {
            bounds,
            resource: image,
        });
    }

    fn backdrop_filter(&mut self, rect: Rect, filter: &floom_types::Filter) {
        let floom_types::Filter::Blur { radius_x, radius_y } = *filter;
        self.scene.insert(SceneNode::BackdropFilter {
            rect,
            filter: *filter,
        });
        self.ops.push(PaintOp::BackdropFilter {
            bounds: rect,
            radius_x,
            radius_y,
        });
    }

    fn animation_value(&self) -> Option<f64> {
        self.animation_value
    }
}

/// Captures what a widget publishes to assistive technology.
pub struct SemanticsRecorder {
    bounds: Rect,
    size_hint: Size,
    label: Option<String>,
    role: SemanticsRole,
    reported_bounds: Option<Rect>,
}

impl SemanticsRecorder {
    pub fn new(bounds: Rect) -> Self {
        Self {
            bounds,
            size_hint: bounds.size(),
            label: None,
            role: SemanticsRole::None,
            reported_bounds: None,
        }
    }

    pub fn with_size_hint(mut self, size_hint: Size) -> Self {
        self.size_hint = size_hint;
        self
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// The role the widget published, if any. Defaults to [`SemanticsRole::None`]
    /// until a widget calls `set_role`.
    pub fn role(&self) -> SemanticsRole {
        self.role
    }

    pub fn reported_bounds(&self) -> Option<Rect> {
        self.reported_bounds
    }

    pub fn snapshot(&self) -> String {
        match (&self.label, self.reported_bounds) {
            (None, None) => "decorative".to_string(),
            (None, Some(bounds)) => format!("unnamed {}", fmt_rect(bounds)),
            (Some(label), None) => format!("label={label:?}"),
            (Some(label), Some(bounds)) => format!("label={label:?} {}", fmt_rect(bounds)),
        }
    }
}

impl SemanticsContext for SemanticsRecorder {
    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn set_label(&mut self, label: &str) {
        self.label = Some(label.to_string());
    }

    fn set_role(&mut self, role: SemanticsRole) {
        self.role = role;
    }

    fn set_bounds(&mut self, bounds: Rect) {
        self.reported_bounds = Some(bounds);
    }

    fn size_hint(&self) -> Size {
        self.size_hint
    }
}

/// Hands out sequential element ids.
#[derive(Default)]
pub struct AllocatingCreateContext {
    next: u32,
}

impl AllocatingCreateContext {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CreateContext for AllocatingCreateContext {
    fn alloc_child(&mut self) -> ElementId {
        let id = ElementId::new(self.next, 1);
        self.next += 1;
        id
    }
}

/// Records every dirty notification.
///
/// Hand-written `Default` rather than derived, for one reason: a derived `Default`
/// sets `overlays_supported` to `false`, because that is what `bool::default()` is.
/// So a context built the obvious way silently refused every overlay, and the first
/// symptom was a state-machine test reporting that nothing had been opened — which
/// looked like a bug in the widget and was a bug in the double. The door being open is
/// the interesting case, so it is what the default means.
pub struct RecordingUpdateContext {
    dirty: Vec<ElementId>,
    /// Nodes whose *children* were declared changed. Kept apart from `dirty`
    /// because the two are not the same request: a repaint re-runs `paint`, a
    /// reconcile re-runs `children()` and can mount or unmount a subtree. A test
    /// that cannot tell them apart cannot pin a stateful widget, because every
    /// state change is supposed to do the second.
    children_dirty: Vec<ElementId>,
    /// Nodes whose `layout_request` moved. A third kind of request, and the one a
    /// stateful widget is most likely to get wrong: a repaint of a box that is
    /// still the old size changes nothing a user can see.
    layout_dirty: Vec<ElementId>,
    /// This node's box, in viewport coordinates. Zero unless set, which is the
    /// honest default: an update context that knows no geometry says so, rather
    /// than reporting an empty rect that happens to look like a real answer.
    bounds: Rect,
    /// The host viewport, for a widget whose overlay spans the window.
    viewport: Rect,
    /// Every overlay asked for through the state-driven door, in order.
    ///
    /// `open_overlay`/`close_overlay` are *defaulted* on `UpdateContext`, so until
    /// this existed a composite that mounted its panel from `update` — which is
    /// exactly what a popup, a dialog and a drawer all do — was untestable here:
    /// the door returned `false` and a test could not tell a deliberate fallback
    /// from a missing implementation. `TestEventContext` already recorded the
    /// event-driven twin; this is the same recording on the other door.
    overlays: Vec<OverlayCall>,
    next_handle: u64,
    /// Whether this context answers like a host with overlay support.
    ///
    /// `false` is a real host, not a broken double: a headless embedder refuses every
    /// request, and the whole point of `OverlayHost` is that a composite falls back
    /// rather than showing nothing. A double that *always* granted would make the
    /// fallback branch unreachable in tests, which is exactly how a fallback rots.
    overlays_supported: bool,
}

impl Default for RecordingUpdateContext {
    fn default() -> Self {
        RecordingUpdateContext {
            dirty: Vec::new(),
            children_dirty: Vec::new(),
            layout_dirty: Vec::new(),
            bounds: Rect::ZERO,
            viewport: Rect::ZERO,
            overlays: Vec::new(),
            next_handle: 0,
            overlays_supported: true,
        }
    }
}

impl RecordingUpdateContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// A context that reports `bounds` for this node.
    ///
    /// The reason this exists: `UpdateContext::bounds` is documented as the
    /// *previous* frame's box, and as "exactly right for anchoring an overlay to a
    /// control that already exists". A test that cannot set it cannot check that a
    /// popup put itself below its trigger.
    pub fn with_bounds(bounds: Rect) -> Self {
        RecordingUpdateContext {
            bounds,
            ..RecordingUpdateContext::default()
        }
    }

    /// A context that also reports `viewport`.
    ///
    /// Separate from [`Self::with_bounds`] because a **dialog** is sized from the
    /// viewport, not from its own box — so a test that sets only the bounds cannot tell
    /// a dialog that covered the window from one that covered itself.
    pub fn with_viewport(bounds: Rect, viewport: Rect) -> Self {
        RecordingUpdateContext {
            bounds,
            viewport,
            ..RecordingUpdateContext::default()
        }
    }

    /// A context that refuses every overlay, as a headless embedder does.
    pub fn without_overlay_support() -> Self {
        RecordingUpdateContext {
            overlays_supported: false,
            ..RecordingUpdateContext::default()
        }
    }

    /// Every overlay call made through the state-driven door, in order.
    pub fn overlays(&self) -> &[OverlayCall] {
        &self.overlays
    }

    /// The overlays asked to be *opened*, in order.
    pub fn opened_overlays(&self) -> Vec<&OverlayCall> {
        self.overlays
            .iter()
            .filter(|call| matches!(call, OverlayCall::Open { .. }))
            .collect()
    }

    /// The handles asked to be *closed*, in order.
    pub fn closed_overlays(&self) -> Vec<OverlayHandle> {
        self.overlays
            .iter()
            .filter_map(|call| match call {
                OverlayCall::Close(handle) => Some(*handle),
                OverlayCall::Open { .. } => None,
            })
            .collect()
    }

    /// Whether any overlay was opened at all — the distinction a bare `false`
    /// from [`UpdateContext::open_overlay`] cannot make on its own.
    pub fn opened_any_overlay(&self) -> bool {
        !self.opened_overlays().is_empty()
    }

    pub fn dirty(&self) -> &[ElementId] {
        &self.dirty
    }

    pub fn dirty_count(&self) -> usize {
        self.dirty.len()
    }

    /// Every id passed to [`UpdateContext::mark_children_dirty`], in order.
    pub fn children_dirty(&self) -> &[ElementId] {
        &self.children_dirty
    }

    /// Whether any node asked to have its children re-declared.
    pub fn reconciled(&self) -> bool {
        !self.children_dirty.is_empty()
    }

    /// Every id passed to [`UpdateContext::mark_layout_dirty`], in order.
    pub fn layout_dirty(&self) -> &[ElementId] {
        &self.layout_dirty
    }

    /// Whether any node asked for the layout pass to run again.
    pub fn laid_out_again(&self) -> bool {
        !self.layout_dirty.is_empty()
    }

    /// Forgets everything recorded, so a second phase of a test can assert what
    /// a *later* update does rather than what the first one did.
    pub fn clear(&mut self) {
        self.dirty.clear();
        self.children_dirty.clear();
        self.layout_dirty.clear();
        self.overlays.clear();
    }
}

impl UpdateContext for RecordingUpdateContext {
    fn mark_dirty(&mut self, id: ElementId) {
        self.dirty.push(id);
    }

    fn mark_children_dirty(&mut self, id: ElementId) {
        self.children_dirty.push(id);
    }

    fn mark_layout_dirty(&mut self, id: ElementId) {
        self.layout_dirty.push(id);
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn viewport(&self) -> Rect {
        self.viewport
    }

    /// Grants the request and records it, running the content factory exactly as
    /// the real host does — so a test sees the content the host would have built,
    /// holding the handle the host would have given it.
    fn open_overlay(&mut self, request: floom_core::OverlayRequest) -> bool {
        if !self.overlays_supported {
            return false;
        }
        let handle = OverlayHandle::new(self.next_handle);
        self.next_handle += 1;
        let content = (request.content)(handle);
        self.overlays.push(OverlayCall::Open {
            handle,
            kind: request.kind,
            modal: request.modal,
            bounds: request.bounds,
            content,
        });
        true
    }

    fn close_overlay(&mut self, handle: OverlayHandle) -> bool {
        self.overlays.push(OverlayCall::Close(handle));
        true
    }
}

/// Dispatches `event` at `widget` inside `bounds`.
///
/// Returns `true` when the widget consumed the event.
pub fn dispatch(widget: &mut dyn Widget, id: ElementId, bounds: Rect, event: Event) -> bool {
    let mut cx = TestEventContext::new(id, bounds, event);
    widget.event(&mut cx, id)
}

/// Minimal [`EventContext`] for driving widgets directly in tests.
pub struct TestEventContext {
    target: ElementId,
    bounds: Rect,
    event: Event,
    /// Every id passed to [`EventContext::request_focus`], in order.
    pub focus_requests: Vec<ElementId>,
    overlays: Vec<OverlayCall>,
    /// Mimics the host's handle counter, so a widget that stores the handle it
    /// was given can be driven through a realistic open/close cycle.
    next_handle: u64,
}

/// An overlay command a widget issued while an event was dispatched.
pub enum OverlayCall {
    Open {
        handle: OverlayHandle,
        kind: OverlayKind,
        modal: bool,
        bounds: Rect,
        /// Already built, exactly as the real host builds it: the content
        /// factory ran with the handle above.
        content: Box<dyn WidgetAny>,
    },
    Close(OverlayHandle),
}

impl OverlayCall {
    pub fn handle(&self) -> OverlayHandle {
        match self {
            OverlayCall::Open { handle, .. } => *handle,
            OverlayCall::Close(handle) => *handle,
        }
    }
}

impl TestEventContext {
    pub fn new(target: ElementId, bounds: Rect, event: Event) -> Self {
        Self {
            target,
            bounds,
            event,
            focus_requests: Vec::new(),
            overlays: Vec::new(),
            next_handle: 0,
        }
    }

    pub fn pointer(target: ElementId, bounds: Rect, pointer: PointerEvent) -> Self {
        Self::new(target, bounds, Event::Pointer(pointer))
    }

    pub fn focus_requests(&self) -> &[ElementId] {
        &self.focus_requests
    }

    /// Overlay commands issued during this dispatch, in order.
    pub fn overlay_calls(&self) -> &[OverlayCall] {
        &self.overlays
    }

    /// The most recent overlay a widget asked to open.
    pub fn last_open(&self) -> Option<&OverlayCall> {
        self.overlays
            .iter()
            .rev()
            .find(|call| matches!(call, OverlayCall::Open { .. }))
    }

    /// Hands the last opened content out of the recorder, so a test can drive
    /// it and then return it through `Widget::overlay_closed` the way the host
    /// would.
    pub fn take_content(&mut self) -> Option<Box<dyn WidgetAny>> {
        let index = self
            .overlays
            .iter()
            .rposition(|call| matches!(call, OverlayCall::Open { .. }))?;
        match &mut self.overlays[index] {
            OverlayCall::Open { content, .. } => {
                Some(std::mem::replace(content, Box::new(NoContent)))
            }
            OverlayCall::Close(_) => None,
        }
    }

    /// Handles a widget asked the host to close.
    pub fn closed_handles(&self) -> Vec<OverlayHandle> {
        self.overlays
            .iter()
            .filter_map(|call| match call {
                OverlayCall::Close(handle) => Some(*handle),
                OverlayCall::Open { .. } => None,
            })
            .collect()
    }
}

/// Stands in for content a test has taken ownership of.
struct NoContent;

impl floom_core::Widget for NoContent {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}
}

impl WidgetAny for NoContent {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

impl EventContext for TestEventContext {
    fn target(&self) -> ElementId {
        self.target
    }

    fn bounds(&self) -> Rect {
        self.bounds
    }

    fn cursor_position(&self) -> Option<Point> {
        match &self.event {
            Event::Pointer(pointer) => Some(pointer.position),
            _ => None,
        }
    }

    fn event(&self) -> Event {
        self.event.clone()
    }

    fn request_focus(&mut self, id: ElementId) {
        self.focus_requests.push(id);
    }

    fn open_overlay(&mut self, request: floom_core::OverlayRequest) -> bool {
        let handle = OverlayHandle::new(self.next_handle);
        self.next_handle += 1;
        let content = (request.content)(handle);
        self.overlays.push(OverlayCall::Open {
            handle,
            kind: request.kind,
            modal: request.modal,
            bounds: request.bounds,
            content,
        });
        true
    }

    fn close_overlay(&mut self, handle: OverlayHandle) -> bool {
        self.overlays.push(OverlayCall::Close(handle));
        true
    }
}

/// Paints `widget` inside `bounds` and returns its scene snapshot.
pub fn paint_snapshot(widget: &dyn Widget, id: ElementId, bounds: Rect) -> String {
    let mut recorder = SceneRecorder::new(bounds);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.snapshot()
}

/// Paints `widget` — typed as `WidgetAny` — inside `bounds` and returns the snapshot.
///
/// The separate entry point is not tidiness. `WidgetAny: Widget`, so the value *is* a
/// widget, but upcasting `&dyn WidgetAny` to `&dyn Widget` needs trait upcasting, which
/// is still unstable — and the thing being painted here is exactly the kind of value a
/// caller cannot name concretely: the content an `OverlayRequest`'s factory built.
///
/// So this calls [`Widget::paint`] through the `WidgetAny` handle directly, which needs no
/// upcast because the method is found on the supertrait.
pub fn paint_snapshot_any(widget: &dyn WidgetAny, id: ElementId, bounds: Rect) -> String {
    let mut recorder = SceneRecorder::new(bounds);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.snapshot()
}

/// Emits `widget` semantics inside `bounds` and returns the snapshot.
pub fn semantics_snapshot(widget: &dyn Widget, id: ElementId, bounds: Rect) -> String {
    let mut recorder = SemanticsRecorder::new(bounds);
    widget.semantics(&mut recorder, id);
    recorder.snapshot()
}
