//! Vello-oriented Scene adapter.
//!
//! Records Scene draw ops into an intermediate command list. Full WGPU/Vello
//! submission continues to live in the legacy `floem_vello_renderer` until the
//! paint path is fully migrated; this crate freezes the ABI boundary.

use floem_renderer::{FrameInfo, Renderer};
use floem_scene::{DamageRegion, Scene, SceneNode};
use floem_types::{Color, Rect};

/// Recorded draw command (Vello backend consumes this later).
#[derive(Debug, Clone, PartialEq)]
pub enum VelloCommand {
    FillRect { bounds: Rect, color: Color },
    DrawImage { bounds: Rect },
    DrawGlyphs { bounds: Rect },
}

#[derive(Debug, Default)]
pub struct VelloRendererAdapter {
    pub commands: Vec<VelloCommand>,
    pub frames: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    /// Simple fingerprint cache of last full-frame command count.
    last_cmd_len: usize,
}

impl VelloRendererAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn commands(&self) -> &[VelloCommand] {
        &self.commands
    }
}

impl Renderer for VelloRendererAdapter {
    fn begin_frame(&mut self, _info: &FrameInfo) {
        self.commands.clear();
    }

    fn render_scene(&mut self, scene: &Scene, damage: &DamageRegion) {
        if damage.is_empty() && self.last_cmd_len > 0 {
            self.cache_hits += 1;
            return;
        }
        self.cache_misses += 1;
        for (_id, node) in scene.iter() {
            match node {
                SceneNode::Rect { bounds, fill } => {
                    self.commands.push(VelloCommand::FillRect {
                        bounds: *bounds,
                        color: *fill,
                    });
                }
                SceneNode::Image { bounds, .. } => {
                    self.commands.push(VelloCommand::DrawImage { bounds: *bounds });
                }
                SceneNode::Text { origin, size, .. } => {
                    self.commands.push(VelloCommand::DrawGlyphs {
                        bounds: Rect::from_origin_size(*origin, *size),
                    });
                }
                SceneNode::Group { .. } => {}
            }
        }
        self.last_cmd_len = self.commands.len();
    }

    fn end_frame(&mut self) {
        self.frames += 1;
    }

    fn trim_caches(&mut self) {
        self.last_cmd_len = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floem_renderer::{FrameInfo, render_once};
    use floem_scene::{Point, SceneNode, Size};
    use floem_types::Color;

    #[test]
    fn records_rect_image_text_fast_paths() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::ZERO, Size::new(10.0, 10.0)),
            fill: Color::BLACK,
        });
        let _ = scene.insert(SceneNode::Image {
            bounds: Rect::from_origin_size(Point::ZERO, Size::new(8.0, 8.0)),
            image: floem_types::ResourceId::new(0, 1),
        });
        let _ = scene.insert(SceneNode::Text {
            origin: Point::ZERO,
            size: Size::new(20.0, 12.0),
            glyphs: floem_types::ResourceId::new(1, 1),
            color: Color::WHITE,
        });
        scene.finish_update();

        let mut vello = VelloRendererAdapter::new();
        render_once(&mut vello, &FrameInfo::software(32, 32, 1), &scene);
        assert_eq!(vello.commands.len(), 3);
        assert!(matches!(vello.commands[0], VelloCommand::FillRect { .. }));
        assert!(matches!(vello.commands[1], VelloCommand::DrawImage { .. }));
        assert!(matches!(vello.commands[2], VelloCommand::DrawGlyphs { .. }));
        assert_eq!(vello.cache_misses, 1);
    }

    #[test]
    fn empty_damage_can_hit_cache() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::ZERO, Size::new(1.0, 1.0)),
            fill: Color::BLACK,
        });
        scene.finish_update();

        let mut vello = VelloRendererAdapter::new();
        render_once(&mut vello, &FrameInfo::software(8, 8, 1), &scene);
        scene.clear_damage();
        render_once(&mut vello, &FrameInfo::software(8, 8, 2), &scene);
        assert!(vello.cache_hits >= 1);
    }
}
