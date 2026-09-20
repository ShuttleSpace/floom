//! Retained scene node payloads (no GPU / OS types).

use floem_types::{
    ClipRegion, Color, Path, Point, Rect, ResourceId, SceneNodeId, Size, Stroke, Transform,
};
use std::sync::Arc;

/// Drawable / structural node in the retained scene.
#[derive(Debug, Clone, PartialEq)]
pub enum SceneNode {
    Group {
        children: smallvec::SmallVec<[SceneNodeId; 4]>,
        transform: Transform,
    },
    Rect {
        bounds: Rect,
        fill: Color,
        /// Corner radius in logical px; `0.0` is a plain right-angle rect.
        /// A single node rather than a separate variant: rounded corners are
        /// the same fill, just a different shape, and every painter wants a
        /// uniform path for the common case.
        radius: f32,
    },
    /// An arbitrary vector path, filled and/or stroked.
    ///
    /// One node rather than two so "fill it, then outline it" stays a single
    /// unit of work with a single position in the draw order — the shape is
    /// flattened once by a backend that needs the segments, and the fill and
    /// the stroke cannot drift apart in the z-order the way two nodes could.
    ///
    /// `fill: None` with a stroke, or the reverse, is normal: a line chart
    /// strokes and does not fill, a chart area fills and does not stroke.
    Path {
        /// Behind an `Arc` for the same reason [`ClipRegion::Path`] is: the
        /// scene node is rebuilt on every frame the widget paints, and a
        /// retained path handed in as a clone of the same `Arc` costs a
        /// pointer bump instead of a deep copy of every segment. An
        /// immediate-mode caller that builds a fresh path each frame pays one
        /// allocation either way.
        path: Arc<Path>,
        fill: Option<Color>,
        stroke: Option<Stroke>,
    },
    /// Pushes a clip region onto the renderer's clip stack.
    ///
    /// The stack, not a folded intersection: the intersection of two rounded
    /// rectangles is not a rounded rectangle, and of two paths is neither, so
    /// a scene that carried one pre-combined region would have to approximate
    /// what a stack applies exactly.
    ///
    /// Pushes and pops are **stream-position, not nesting** — they arrive in
    /// the order the host painted them, and a well-formed scene never pops an
    /// empty stack. The walk may, however, *end* with regions still pushed: a
    /// renderer must therefore begin every walk with an empty stack rather than
    /// carrying one over from the previous frame. Nothing is drawn after the
    /// last node, so a trailing push has no visual effect; inheriting it would.
    ClipPush { region: ClipRegion },
    /// Pops one clip region. See [`SceneNode::ClipPush`].
    ClipPop,
    Image {
        bounds: Rect,
        image: ResourceId,
        /// Per-node alpha, `255` meaning opaque.
        ///
        /// An image carries its own pixels and no tint, which is what made
        /// `Rect`-style fading impossible here for a long time — and why an
        /// overlay containing an image used to fade around it. The composite
        /// path scales this exactly the way it scales a `Rect`'s `fill.a`.
        alpha: u8,
    },
    Text {
        origin: Point,
        size: Size,
        glyphs: ResourceId,
        color: Color,
    },
}

impl SceneNode {
    /// An opaque image filling `bounds`.
    ///
    /// Every construction site wants 255, and writing a literal at each one is
    /// how a future field gets forgotten in half of them.
    pub fn image(bounds: Rect, image: ResourceId) -> Self {
        Self::Image {
            bounds,
            image,
            alpha: u8::MAX,
        }
    }

    /// A filled path. Takes anything that becomes an `Arc<Path>`, so a caller
    /// that already holds one (a widget that kept its geometry) shares it and
    /// a caller that just built one hands it over.
    pub fn filled_path(path: impl Into<Arc<Path>>, fill: Color) -> Self {
        Self::Path {
            path: path.into(),
            fill: Some(fill),
            stroke: None,
        }
    }

    /// A stroked path.
    pub fn stroked_path(path: impl Into<Arc<Path>>, stroke: Stroke) -> Self {
        Self::Path {
            path: path.into(),
            fill: None,
            stroke: Some(stroke),
        }
    }

    /// The node's painted extent, when it has one.
    ///
    /// `None` for the structural variants — `Group`, and the clip markers.
    /// This is not "unknown": it is the statement "this node paints nothing of
    /// its own, and what it affects is bounded by its children". Both consumers
    /// read it that way — a node with no hint is never culled (it may still
    /// have to run for its children, or for the clip stack), and a *change* to
    /// one is treated as full-frame damage rather than a guess at a region.
    pub fn bounds_hint(&self) -> Option<Rect> {
        match self {
            SceneNode::Group { .. } => None,
            SceneNode::Rect { bounds, .. } => Some(*bounds),
            SceneNode::Path { path, fill, stroke } => {
                if fill.is_none() && stroke.is_none() {
                    return None;
                }
                let mut bounds = path.bounds();
                if let Some(stroke) = stroke {
                    // A miter can reach `miter_limit` half-widths out; anything
                    // less would report damage that misses the spike the
                    // stroke actually painted.
                    let reach = match stroke.join {
                        floem_types::LineJoin::Miter => stroke.half_width() * stroke.miter_limit,
                        _ => stroke.half_width(),
                    };
                    bounds = bounds.union(Rect {
                        x0: bounds.x0 - reach,
                        y0: bounds.y0 - reach,
                        x1: bounds.x1 + reach,
                        y1: bounds.y1 + reach,
                    });
                }
                Some(bounds)
            }
            SceneNode::ClipPush { .. } | SceneNode::ClipPop => None,
            SceneNode::Image { bounds, .. } => Some(*bounds),
            SceneNode::Text { origin, size, .. } => Some(Rect::from_origin_size(*origin, *size)),
        }
    }
}
