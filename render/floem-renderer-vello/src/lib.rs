//! Vello-oriented Scene adapter.
//!
//! Two layers live here:
//!
//! * [`VelloRendererAdapter`] — a dependency-free recorder that turns a
//!   [`Scene`] into a [`VelloCommand`] list. It freezes the ABI boundary and is
//!   what the default build compiles.
//! * [`VelloGpuPresenter`] (feature `gpu`) — the real thing: a wgpu
//!   swap-chain driven by vello, which is what actually puts pixels in a window.

#[cfg(feature = "gpu")]
mod gpu;

#[cfg(feature = "gpu")]
pub use gpu::{GpuFrameStats, VelloGpuPresenter};

use floem_renderer::{FrameInfo, Renderer};
use floem_scene::{DamageRegion, Scene, SceneNode};
use floem_types::{Color, Rect};

/// Recorded draw command (Vello backend consumes this later).
#[derive(Debug, Clone, PartialEq)]
pub enum VelloCommand {
    FillRect {
        bounds: Rect,
        color: Color,
        /// Corner radius in logical px; `0.0` is a plain right-angle rect.
        radius: f32,
    },
    DrawImage {
        bounds: Rect,
    },
    DrawGlyphs {
        bounds: Rect,
    },
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
                SceneNode::Rect {
                    bounds,
                    fill,
                    radius,
                } => {
                    self.commands.push(VelloCommand::FillRect {
                        bounds: *bounds,
                        color: *fill,
                        radius: radius.max(0.0),
                    });
                }
                SceneNode::Image { bounds, .. } => {
                    self.commands
                        .push(VelloCommand::DrawImage { bounds: *bounds });
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
    use floem_renderer::{render_once, FrameInfo};
    use floem_scene::{Point, SceneNode, Size};
    use floem_types::Color;

    #[test]
    fn records_rect_image_text_fast_paths() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::ZERO, Size::new(10.0, 10.0)),
            fill: Color::BLACK,
            radius: 0.0,
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
            radius: 0.0,
        });
        scene.finish_update();

        let mut vello = VelloRendererAdapter::new();
        render_once(&mut vello, &FrameInfo::software(8, 8, 1), &scene);
        scene.clear_damage();
        render_once(&mut vello, &FrameInfo::software(8, 8, 2), &scene);
        assert!(vello.cache_hits >= 1);
    }

    /// The adapter forwards the corner radius verbatim so the GPU presenter
    /// can shape corners from the command list alone.
    #[test]
    fn fill_rect_commands_carry_their_radius() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::ZERO, Size::new(10.0, 10.0)),
            fill: Color::BLACK,
            radius: 6.0,
        });
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(0.0, 20.0), Size::new(10.0, 10.0)),
            fill: Color::BLACK,
            radius: -3.0, // clamped to 0 at the adapter boundary
        });
        scene.finish_update();

        let mut vello = VelloRendererAdapter::new();
        render_once(&mut vello, &FrameInfo::software(32, 32, 1), &scene);
        let [a, b] = vello.commands.as_slice() else {
            panic!("expected exactly two commands");
        };
        let (VelloCommand::FillRect { radius: ra, .. }, VelloCommand::FillRect { radius: rb, .. }) =
            (a, b)
        else {
            panic!("expected fill-rect commands, got {a:?} / {b:?}");
        };
        assert_eq!(*ra, 6.0);
        assert_eq!(*rb, 0.0, "negative radius is clamped");
    }
}
