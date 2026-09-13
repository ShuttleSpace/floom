//! Retained scene node payloads (no GPU / OS types).

use floem_types::{Color, Point, Rect, ResourceId, SceneNodeId, Size, Transform};

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
    Image {
        bounds: Rect,
        image: ResourceId,
    },
    Text {
        origin: Point,
        size: Size,
        glyphs: ResourceId,
        color: Color,
    },
}

impl SceneNode {
    pub fn bounds_hint(&self) -> Option<Rect> {
        match self {
            SceneNode::Group { .. } => None,
            SceneNode::Rect { bounds, .. } => Some(*bounds),
            SceneNode::Image { bounds, .. } => Some(*bounds),
            SceneNode::Text { origin, size, .. } => Some(Rect::from_origin_size(*origin, *size)),
        }
    }
}
