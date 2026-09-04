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
            SceneNode::Text { origin, size, .. } => {
                Some(Rect::from_origin_size(*origin, *size))
            }
        }
    }
}
