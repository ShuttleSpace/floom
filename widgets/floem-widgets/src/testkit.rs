//! Deterministic recorders for widget snapshot tests.
//!
//! These implement the [`floem_core`] context traits and capture what a widget
//! emits, so primitives can be asserted against a stable textual snapshot
//! without a renderer or a window. The paint recorder additionally lowers every
//! op into a real [`Scene`], which proves the widget output can be retained.

use floem_core::{
    Color, CreateContext, ElementId, PaintContext, Point, Rect, ResourceId, SemanticsContext, Size,
    UpdateContext, Widget,
};
use floem_scene::{Resource, ResourceKind, ResourceRegistry, Scene, SceneNode};

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
        color: Color,
    },
    Image {
        bounds: Rect,
        resource: ResourceId,
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
                color,
            } => format!(
                "Text ({:.1}, {:.1}) {:.1} {:?} {}",
                origin.x,
                origin.y,
                size,
                text,
                fmt_color(*color)
            ),
            PaintOp::Image { bounds, resource } => format!(
                "Image {} res=({}, {})",
                fmt_rect(*bounds),
                resource.index(),
                resource.generation()
            ),
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
        }
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

    /// Node kind name for each retained scene node, in insertion order.
    pub fn scene_node_kinds(&self) -> Vec<&'static str> {
        self.scene
            .iter()
            .map(|(_, node)| match node {
                SceneNode::Group { .. } => "Group",
                SceneNode::Rect { .. } => "Rect",
                SceneNode::Image { .. } => "Image",
                SceneNode::Text { .. } => "Text",
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
            fill: color,
        });
        self.ops.push(PaintOp::Rect {
            bounds: rect,
            color,
        });
    }

    fn fill_rounded_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        // The retained scene has no rounded-rect node yet; record the filled
        // rect and keep the radius in the op list for snapshotting.
        self.scene.insert(SceneNode::Rect {
            bounds: rect,
            fill: color,
        });
        self.ops.push(PaintOp::RoundedRect {
            bounds: rect,
            radius,
            color,
        });
    }

    fn fill_text(&mut self, origin: Point, text: &str, size: f32, color: Color) {
        let glyphs = self.registry.insert(Resource {
            kind: ResourceKind::GlyphRun {
                glyph_count: text.chars().count() as u32,
            },
        });
        // No shaping happens here (`floem-text` owns that), so the retained
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
            color,
        });
    }

    fn draw_image(&mut self, bounds: Rect, image: ResourceId) {
        self.scene.insert(SceneNode::Image { bounds, image });
        self.ops.push(PaintOp::Image {
            bounds,
            resource: image,
        });
    }
}

/// Captures what a widget publishes to assistive technology.
pub struct SemanticsRecorder {
    bounds: Rect,
    size_hint: Size,
    label: Option<String>,
    reported_bounds: Option<Rect>,
}

impl SemanticsRecorder {
    pub fn new(bounds: Rect) -> Self {
        Self {
            bounds,
            size_hint: bounds.size(),
            label: None,
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
#[derive(Default)]
pub struct RecordingUpdateContext {
    dirty: Vec<ElementId>,
}

impl RecordingUpdateContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dirty(&self) -> &[ElementId] {
        &self.dirty
    }

    pub fn dirty_count(&self) -> usize {
        self.dirty.len()
    }
}

impl UpdateContext for RecordingUpdateContext {
    fn mark_dirty(&mut self, id: ElementId) {
        self.dirty.push(id);
    }
}

/// Paints `widget` inside `bounds` and returns its scene snapshot.
pub fn paint_snapshot(widget: &dyn Widget, id: ElementId, bounds: Rect) -> String {
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
